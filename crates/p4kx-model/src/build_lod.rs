// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_chunks::ivo;

use crate::bone_map::check_joints;
use crate::build::{MAX_LOD, Rig, corrupt, fetch, first_of, read_meshes, read_skeleton, usage};
use crate::companion::lod_file_name;
use crate::model::MESH_TYPES;
use crate::{Bone, CompanionResolver, Lod, LodSelection, ModelChunk, ModelError, ModelFile};

/// Reads the skeleton of the model file of `level`, which a skinned level's bone indices refer
/// to; `None` when the file is missing, holds no skeleton or the one `base` already is.
fn level_skeleton(
    name: &str,
    level: u32,
    base: &[Bone],
    companions: &mut dyn CompanionResolver,
) -> Result<Option<Vec<Bone>>, ModelError> {
    let file_name = lod_file_name(name, level);
    let Some(file) = fetch(companions, &file_name)? else {
        return Ok(None);
    };
    let bones = read_skeleton(&file_name, &file, &mut Vec::new())?;
    Ok((!bones.is_empty() && bones != base).then_some(bones))
}

/// Reads the geometry companions `<stem>_lod<n>.<extension>m` of the levels from 1 on that
/// `selection` wants; a level without companion is left out.
pub(crate) fn read_lod_companions(
    name: &str,
    companions: &mut dyn CompanionResolver,
    selection: LodSelection,
    rig: Rig<'_>,
    lods: &mut Vec<Lod>,
    chunks: &mut Vec<ModelChunk>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<(), ModelError> {
    for level in (1..=MAX_LOD).filter(|&level| selection.wants(level)) {
        let companion_name = format!("{}m", lod_file_name(name, level));
        let Some(companion) = fetch(companions, &companion_name)? else {
            continue;
        };
        let Some(index) = first_of(&companion, &MESH_TYPES) else {
            if companion.chunks_of(ivo::VIS_AREAS).next().is_some() {
                continue;
            }
            return Err(ModelError::NoMesh {
                file: companion_name,
            });
        };
        read.push((ModelFile::LodGeometry(level), index));
        let meshes = read_meshes(
            &companion_name,
            index,
            companion.chunk_data(index),
            rig,
            false,
        )?;
        let bones = if meshes.iter().any(|mesh| mesh.influences.is_some()) {
            level_skeleton(name, level, rig.bones, companions)?
        } else {
            None
        };
        let count = bones.as_ref().map_or(rig.bones.len(), Vec::len);
        check_joints(&meshes, count).map_err(corrupt(&companion_name, index, "Skin"))?;
        lods.push(Lod {
            level,
            meshes,
            bones,
        });
        chunks.extend(usage(&companion, ModelFile::LodGeometry(level), read));
    }
    Ok(())
}
