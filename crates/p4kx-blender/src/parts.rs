// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use p4kx_assembly::Assembly;
use p4kx_gltf::{GlbOptions, GlbSummary, GltfError, Scene, SceneNode, ScenePart, write_scene_glb};
use p4kx_model::Transform;
use rayon::prelude::*;

use crate::plan::{PartPlan, part_id};
use crate::{Cancelled, PackageError, PackageResult};

/// Writes the GLB file `<folder>/<id>.glb` of every distinct part of `plan`, in parallel.
pub(crate) fn write_all(
    assembly: &Assembly,
    plan: &PartPlan,
    options: &GlbOptions,
    folder: &Path,
    cancelled: &Cancelled<'_>,
) -> PackageResult<Vec<GlbSummary>> {
    (0..plan.len())
        .into_par_iter()
        .map(|index| {
            cancelled.check()?;
            write_one(assembly, plan, index, options, folder)
        })
        .collect()
}

fn write_one(
    assembly: &Assembly,
    plan: &PartPlan,
    index: usize,
    options: &GlbOptions,
    folder: &Path,
) -> PackageResult<GlbSummary> {
    let id = part_id(index);
    let first = plan
        .first(index)
        .and_then(|position| assembly.parts().get(position));
    let mut scene = Scene::new();
    let mut name = String::new();
    if let Some(part) = first
        && let Some(model) = part.model()
    {
        part.class_name().clone_into(&mut name);
        let drawn = ScenePart::new(model, part.materials());
        let drawn = part
            .palette()
            .map_or(drawn, |palette| drawn.with_palette(palette));
        let node = SceneNode::new(part.class_name(), None, Transform::IDENTITY).with_part(drawn);
        scene
            .push(node)
            .map_err(|source| part_error(&id, &name, source))?;
    }
    let path = folder.join(format!("{id}.glb"));
    let file = File::create_new(&path).map_err(|source| PackageError::write(&path, source))?;
    let mut writer = BufWriter::new(file);
    let summary = write_scene_glb(&scene, options, &mut writer).map_err(|error| match error {
        GltfError::Write(source) => PackageError::write(&path, source),
        source => part_error(&id, &name, source),
    })?;
    writer
        .flush()
        .map_err(|source| PackageError::write(&path, source))?;
    Ok(summary)
}

fn part_error(id: &str, name: &str, source: GltfError) -> PackageError {
    PackageError::Part {
        id: id.to_owned(),
        name: name.to_owned(),
        source,
    }
}
