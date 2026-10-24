// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::Write;

use gltf_json as json;
use json::validation::USize64;
use p4kx_model::{Bone, BoneMatch, Influences, MaterialStatus, Model};

use crate::GltfError;
use crate::document::{GENERATOR, NodeMeshes, push_accessor, push_meshes};
use crate::glb::{GlbOptions, GlbSummary, write_file};
use crate::glb_plan::summarise;
use crate::instance::{Instance, materials};
use crate::layout::{Layout, Rigging};
use crate::materials::MaterialPlan;
use crate::morphs::MorphTarget;
use crate::nodes::{RigModel, push_rig_nodes};
use crate::scene::nearest_lod;
use crate::skin::inverse_binds;
use crate::texture_plan::TextureSet;

/// A skinned model worn on a shared skeleton, with the material library it is drawn with.
///
/// The model's bones are matched to the skeleton by name, see [`BoneMatch`]; its vertices are in
/// the skeleton's space, as the skins of a character are.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RigPiece<'m> {
    /// The name of the piece's node, e.g. `Armor_Torso/rsi_odyssey_undersuit_01`.
    pub name: &'m str,
    /// The model of the piece.
    pub model: &'m Model,
    /// The material library of the model, see [`MaterialStatus::library`].
    pub materials: &'m MaterialStatus,
    /// The morph targets of the piece's first mesh, see [`RigPiece::with_morphs`].
    pub morphs: &'m [MorphTarget],
}

impl<'m> RigPiece<'m> {
    /// Places `model`, drawn with the library of `materials`, in a node called `name`.
    #[must_use]
    pub const fn new(name: &'m str, model: &'m Model, materials: &'m MaterialStatus) -> Self {
        Self {
            name,
            model,
            materials,
            morphs: &[],
        }
    }

    /// Gives the piece's first mesh the morph targets `morphs`, as sparse `POSITION` targets named
    /// by the mesh's `extras.targetNames`, all with the weight 0.
    ///
    /// The vertices of a target index the mesh, whose vertices the piece's skin shapes; a target
    /// that moves no vertex or one the mesh lacks, or whose deltas are not finite, is left out.
    /// They deform the mesh before the skin does, so they combine with the skeleton.
    #[must_use]
    pub const fn with_morphs(mut self, morphs: &'m [MorphTarget]) -> Self {
        self.morphs = morphs;
        self
    }
}

/// Writes `pieces` worn on `skeleton` as one GLB file to `out`; `out` should be buffered.
///
/// The bones become nodes once, one glTF skin over them has the inverse bind matrices of
/// `skeleton`, and every skinned mesh of every piece uses that skin with its bones turned into the
/// skeleton's by name; a bone the skeleton lacks takes its nearest ancestor that it has. A piece
/// whose model has no skinned mesh is placed like in [`write_scene_glb`](crate::write_scene_glb).
/// The materials are plain colours and equal materials are written once. Each piece is drawn at
/// the level of detail `options` asks for, or at the nearest lower one it has;
/// [`GlbSummary::lod_fallbacks`] counts those.
///
/// # Errors
///
/// Returns [`GltfError::TooLarge`] when the pieces do not fit in a GLB file and
/// [`GltfError::Write`] when `out` fails.
pub fn write_rigged_glb(
    skeleton: &[Bone],
    pieces: &[RigPiece<'_>],
    options: &GlbOptions,
    out: &mut impl Write,
) -> Result<GlbSummary, GltfError> {
    let (instances, fallbacks) = instances_of(pieces, options);
    let binds = inverse_binds(skeleton);
    let remapped: Vec<Vec<Vec<Influences>>> = instances
        .iter()
        .map(|instance| remap(skeleton, instance))
        .collect();
    let mut layout = Layout::new();
    for ((part, instance), own) in instances.iter().enumerate().zip(&remapped) {
        if let Some(lod) = instance.lod {
            let rigging = Rigging {
                binds: (!binds.is_empty()).then_some(binds.as_slice()),
                influences: Some(own.as_slice()),
                morphs: pieces.get(part).map_or(&[], |piece| piece.morphs),
            };
            layout.add_part(part, lod, options.vertex_colours, &instance.plan, rigging);
        }
    }
    let images = TextureSet::default();
    let root = document(skeleton, pieces, &instances, &layout, &images);
    let total = write_file(&root, &layout, &images, out)?;
    let defaulted = instances
        .iter()
        .map(|instance| instance.plan.defaulted())
        .sum();
    let drawn_nodes = instances
        .iter()
        .map(|instance| instance.model.nodes.len())
        .sum();
    let mut summary = summarise(&root, &layout, defaulted, drawn_nodes, images, &[], total);
    summary.parts = instances.len();
    summary.lod_fallbacks = fallbacks;
    Ok(summary)
}

/// Returns an instance per piece at the level of detail `options` asks for or the nearest lower
/// one, and the number of pieces that lack it.
fn instances_of<'m>(pieces: &[RigPiece<'m>], options: &GlbOptions) -> (Vec<Instance<'m>>, usize) {
    let mut fallbacks = 0;
    let instances = pieces
        .iter()
        .map(|piece| {
            let lod = nearest_lod(piece.model, options.lod);
            if lod.is_some_and(|lod| lod.level != options.lod) {
                fallbacks += 1;
            }
            Instance {
                model: piece.model,
                lod,
                wrapper: None,
                plan: lod.map_or_else(MaterialPlan::empty, |lod| {
                    MaterialPlan::new(piece.materials.library(), lod)
                }),
                twin: None,
            }
        })
        .collect();
    (instances, fallbacks)
}

/// Builds the glTF document: materials, meshes, and the nodes of the pieces and the skeleton.
fn document(
    skeleton: &[Bone],
    pieces: &[RigPiece<'_>],
    instances: &[Instance<'_>],
    layout: &Layout<'_>,
    images: &TextureSet,
) -> json::Root {
    let mut root = json::Root {
        asset: json::Asset {
            generator: Some(GENERATOR.to_owned()),
            ..json::Asset::default()
        },
        ..json::Root::default()
    };
    let buffer = (layout.len > 0).then(|| {
        root.push(json::Buffer {
            byte_length: USize64(layout.len),
            name: None,
            uri: None,
            extensions: None,
            extras: json::extras::Void::default(),
        })
    });
    let ids = materials(&mut root, layout, instances, images, &[]);
    let (node_meshes, skinned): (NodeMeshes, Vec<Vec<bool>>) = buffer.map_or_else(
        || {
            let meshes = instances
                .iter()
                .map(|instance| vec![None; instance.model.nodes.len()])
                .collect();
            let flags = instances
                .iter()
                .map(|instance| vec![false; instance.model.nodes.len()])
                .collect();
            (meshes, flags)
        },
        |buffer| push_meshes(&mut root, buffer, layout, instances, &ids),
    );
    let bind_accessor = buffer
        .zip(
            layout
                .inverse_binds
                .and_then(|block| layout.blocks.get(block)),
        )
        .map(|(buffer, block)| push_accessor(&mut root, buffer, block, None));
    let any_skinned = skinned.iter().flatten().any(|&skinned| skinned);
    let models: Vec<RigModel<'_>> = pieces
        .iter()
        .zip(&node_meshes)
        .zip(&skinned)
        .map(|((piece, meshes), skinned)| RigModel {
            name: piece.name,
            model: piece.model,
            meshes,
            skinned,
        })
        .collect();
    let bones: &[Bone] = if any_skinned && bind_accessor.is_some() {
        skeleton
    } else {
        &[]
    };
    push_rig_nodes(&mut root, &models, bones, bind_accessor);
    root
}

/// Returns the influences of each mesh of the instance's level with the bones of `skeleton`.
fn remap(skeleton: &[Bone], instance: &Instance<'_>) -> Vec<Vec<Influences>> {
    let Some(lod) = instance.lod else {
        return Vec::new();
    };
    let matched = BoneMatch::new(skeleton, instance.model.bones_of(lod));
    lod.meshes
        .iter()
        .map(|mesh| {
            mesh.influences
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|influences| influences.remapped(|joint| matched.target(joint)))
                .collect()
        })
        .collect()
}
