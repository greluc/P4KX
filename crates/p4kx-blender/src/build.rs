// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::Assembly;
use p4kx_gltf::{DrawnMaterial, GlbOptions, GlbSummary, TextureFile, node_transform};

use crate::PackageInfo;
use crate::collect::Collected;
use crate::manifest::{
    Basis, Coordinates, EntityRef, FORMAT_VERSION, Format, Generator, Issue, IssueOrigin,
    MESHES_FOLDER, Manifest, MaterialTextures, Mesh, MeshMaterial, Node, NodeKind, Options, Source,
    TEXTURES_FOLDER, Texture, TextureFormat, Unit,
};
use crate::plan::{MeshPlan, mesh_id};
use crate::textures::{Outcome, id_of};

/// Everything the manifest is made of.
pub(crate) struct Inputs<'a> {
    pub(crate) assembly: &'a Assembly,
    pub(crate) info: &'a PackageInfo,
    pub(crate) options: &'a GlbOptions,
    pub(crate) plan: &'a MeshPlan,
    pub(crate) collected: &'a Collected,
    pub(crate) outcomes: &'a [Outcome],
    pub(crate) meshes: &'a [GlbSummary],
}

/// Builds the manifest of a package whose files are written.
pub(crate) fn manifest(inputs: &Inputs<'_>) -> Manifest {
    let root = inputs.assembly.root();
    Manifest {
        format: Format::P4kxBlenderPackage,
        version: FORMAT_VERSION.to_owned(),
        generator: Generator {
            name: "p4kx".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
        source: Source {
            channel: inputs.info.channel.clone(),
            build: inputs.info.build.clone(),
            entity: root.map_or_else(
                || EntityRef {
                    class_name: String::new(),
                    guid: String::new(),
                },
                |part| EntityRef {
                    class_name: part.class_name().to_owned(),
                    guid: part.guid().to_string(),
                },
            ),
        },
        coordinates: Coordinates {
            basis: Basis::GltfYUp,
            unit: Unit::Metre,
        },
        options: Options {
            lod: inputs.options.lod,
            texture_mip: inputs.options.texture_mip,
            vertex_colours: inputs.options.vertex_colours,
            exclude: inputs.info.exclude.clone(),
            replace: inputs.info.replace.clone(),
        },
        nodes: nodes(inputs),
        meshes: meshes(inputs),
        textures: textures(inputs),
        issues: issues(inputs),
    }
}

fn nodes(inputs: &Inputs<'_>) -> Vec<Node> {
    inputs
        .assembly
        .parts()
        .iter()
        .enumerate()
        .map(|(position, part)| {
            let transform = node_transform(part.local());
            Node {
                kind: NodeKind::Entity,
                name: part.name().to_owned(),
                path: part.path().to_owned(),
                parent: part.parent(),
                entity: Some(EntityRef {
                    class_name: part.class_name().to_owned(),
                    guid: part.guid().to_string(),
                }),
                mesh: inputs.plan.drawn_by(position).map(mesh_id),
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
            }
        })
        .collect()
}

fn meshes(inputs: &Inputs<'_>) -> Vec<Mesh> {
    inputs
        .meshes
        .iter()
        .enumerate()
        .map(|(index, summary)| {
            let first = inputs
                .plan
                .first(index)
                .and_then(|position| inputs.assembly.parts().get(position));
            let id = mesh_id(index);
            Mesh {
                file: format!("{MESHES_FOLDER}/{id}.glb"),
                id,
                geometry: first
                    .and_then(p4kx_assembly::Part::geometry)
                    .map(str::to_owned),
                palette: first
                    .and_then(p4kx_assembly::Part::palette)
                    .map(|palette| palette.name.clone()),
                instances: inputs.plan.instances(index),
                bytes: summary.bytes,
                vertices: summary.vertices,
                triangles: summary.triangles,
                materials: inputs
                    .collected
                    .materials
                    .get(index)
                    .map(|drawn| {
                        drawn
                            .iter()
                            .enumerate()
                            .map(|(index, material)| mesh_material(inputs, index, material))
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn written_id(inputs: &Inputs<'_>, file: Option<&TextureFile>) -> Option<String> {
    let position = inputs.collected.position_of(&file?.path)?;
    matches!(inputs.outcomes.get(position), Some(Outcome::Written { .. })).then(|| id_of(position))
}

fn mesh_material(inputs: &Inputs<'_>, index: usize, material: &DrawnMaterial) -> MeshMaterial {
    let normal = written_id(inputs, material.normal.as_ref());
    MeshMaterial {
        index,
        name: material.name.clone(),
        textures: MaterialTextures {
            diffuse: written_id(inputs, material.diffuse.as_ref()),
            normal,
            specular: written_id(inputs, material.specular.as_ref()),
        },
    }
}

fn textures(inputs: &Inputs<'_>) -> Vec<Texture> {
    inputs
        .collected
        .wanted
        .iter()
        .zip(inputs.outcomes)
        .enumerate()
        .filter_map(|(position, (wanted, outcome))| match outcome {
            Outcome::Written {
                width,
                height,
                colour_space,
                alpha,
                ..
            } => {
                let id = id_of(position);
                Some(Texture {
                    file: format!("{TEXTURES_FOLDER}/{id}.png"),
                    id,
                    source: wanted.path.clone(),
                    format: TextureFormat::Png,
                    bit_depth: 8,
                    colour_space: *colour_space,
                    alpha: *alpha,
                    width: *width,
                    height: *height,
                })
            }
            Outcome::Failed(_) => None,
        })
        .collect()
}

fn issues(inputs: &Inputs<'_>) -> Vec<Issue> {
    let mut issues: Vec<Issue> = inputs
        .assembly
        .issues()
        .iter()
        .map(|issue| Issue {
            origin: IssueOrigin::Assembly,
            kind: issue.kind().code().to_owned(),
            path: issue.path().to_owned(),
            name: issue.name().to_owned(),
            skipped: issue.kind().skips(),
            reason: issue.kind().to_string(),
        })
        .collect();
    for (wanted, outcome) in inputs.collected.wanted.iter().zip(inputs.outcomes) {
        if let Outcome::Failed(reason) = outcome {
            issues.push(Issue {
                origin: IssueOrigin::Texture,
                kind: "texture".to_owned(),
                path: wanted.file.clone(),
                name: wanted.material.clone(),
                skipped: false,
                reason: reason.clone(),
            });
        }
    }
    let mut refused: Vec<(&str, &str)> = Vec::new();
    for material in inputs.collected.materials.iter().flatten() {
        for (_, file) in &material.refused {
            if !refused.contains(&(file.as_str(), material.name.as_str())) {
                refused.push((file.as_str(), material.name.as_str()));
                issues.push(Issue {
                    origin: IssueOrigin::Texture,
                    kind: "texture_path".to_owned(),
                    path: file.clone(),
                    name: material.name.clone(),
                    skipped: false,
                    reason: "the texture file name is not a safe path".to_owned(),
                });
            }
        }
    }
    issues
}
