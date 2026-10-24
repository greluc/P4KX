// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::{Dna, DnaRig};
use p4kx_model::{Mesh, Model};

use super::normals::carry_normals;
use crate::{ModelFailure, ModelProvider};

/// The largest DNA rig that is read, 256 MiB; the rig of a human head is about 76 MB.
pub const MAX_DNA_RIG_LEN: u64 = 256 << 20;

/// Returns the path of the DNA rig of the skin at `path` for the level of detail `level`: the
/// file beside the skin with the extension `.dna`, and `_lod<level>` before it from level 1 on.
fn rig_path(path: &str, level: u32) -> Option<String> {
    let (stem, extension) = path.rsplit_once('.')?;
    if !extension.eq_ignore_ascii_case("skin") {
        return None;
    }
    Some(if level == 0 {
        format!("{stem}.dna")
    } else {
        format!("{stem}_lod{level}.dna")
    })
}

/// Returns `model`, the skin at `path`, with its vertices moved to the shape `dna` describes.
///
/// The moves are the neutral morphs of the DNA rig beside the skin, mixed by the DNA's weights;
/// the normals follow the new surface.
pub(super) fn shaped(
    model: &Model,
    path: &str,
    dna: &Dna,
    models: &dyn ModelProvider,
) -> Result<Model, String> {
    let level = model
        .lods
        .first()
        .map(|lod| lod.level)
        .ok_or("the model has no level of detail")?;
    let rig_path = rig_path(path, level).ok_or("the model is no skin, it has no DNA rig")?;
    let bytes = models
        .file(&rig_path, MAX_DNA_RIG_LEN)
        .map_err(|failure| match failure {
            ModelFailure::Missing => format!("the DNA rig {rig_path} does not exist"),
            other => format!("the DNA rig {rig_path} cannot be read: {other}"),
        })?;
    let rig = DnaRig::read(&bytes).map_err(|error| format!("{rig_path}: {error}"))?;
    if rig.header().rig_hash != dna.gender_hash {
        return Err(format!(
            "the DNA is for the rig {:#010x}, {rig_path} is the rig {:#010x}",
            dna.gender_hash,
            rig.header().rig_hash
        ));
    }
    let failed = |error| format!("{rig_path}: {error}");
    let matrix = rig.matrix(dna).map_err(failed)?;
    let moves = rig
        .neutral_morphs()
        .and_then(|morphs| morphs.decode(&matrix))
        .map_err(failed)?;
    let mut shaped = model.clone();
    for lod in shaped.lods.iter_mut().take(1) {
        for mesh in &mut lod.meshes {
            apply(mesh, &moves).map_err(|vertex| {
                format!(
                    "{rig_path} covers {} vertices, the model uses the vertex {vertex}",
                    moves.len()
                )
            })?;
        }
    }
    Ok(shaped)
}

/// Moves the vertices of `mesh` by the moves of their stream vertices; returns the stream vertex
/// the moves lack.
fn apply(mesh: &mut Mesh, moves: &[[f32; 3]]) -> Result<(), u32> {
    let old = mesh.positions.clone();
    for (position, &source) in mesh.positions.iter_mut().zip(&mesh.source) {
        let by = usize::try_from(source)
            .ok()
            .and_then(|at| moves.get(at))
            .ok_or(source)?;
        for (value, delta) in position.iter_mut().zip(by) {
            *value += delta;
        }
    }
    carry_normals(mesh, &old);
    Ok(())
}
