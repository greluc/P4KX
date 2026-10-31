// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The face of a story character's own head: the joint deltas of its `.RigLogic` file and the
//! blendshapes of its skin geometry, for a head that ships no DNA rig.

use std::collections::HashMap;

use p4kx_character::{FaceRig, RigLogic, SkinBlendshapes};
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_model::{Bone, Model};

use super::expressions::{Expressions, Morphs, targets_of};
use super::facial::{bones_of, own_facial};
use super::shape::{Outcome, outcome};
use crate::{ModelFailure, ModelProvider};

/// The folder below `Data` that holds the `.RigLogic` file of every head rig.
const LOGIC_FOLDER: &str = "Objects/Characters/RigLogic";
const HEAD_SUFFIX: &str = "_head.skin";
/// The largest `.RigLogic` file and head geometry file that is read, 64 MiB.
const MAX_FILE_LEN: u64 = 64 << 20;

/// Returns the path of the `.RigLogic` file of the head whose skin is at `skin`: the file named
/// like the skin without `_head.skin` in [`LOGIC_FOLDER`]; `None` for any other skin.
pub(super) fn logic_path(skin: &str) -> Option<String> {
    let name = skin.rsplit(['/', '\\']).next()?;
    let cut = name.len().checked_sub(HEAD_SUFFIX.len())?;
    let (stem, tail) = (name.get(..cut)?, name.get(cut..)?);
    (!stem.is_empty() && tail.eq_ignore_ascii_case(HEAD_SUFFIX))
        .then(|| format!("{LOGIC_FOLDER}/{stem}.RigLogic"))
}

/// Reads the `.RigLogic` file at `path`; `None` when the Archive has no such file.
fn read_logic(path: &str, models: &dyn ModelProvider) -> Result<Option<RigLogic>, String> {
    let bytes = match models.file(path, MAX_FILE_LEN) {
        Ok(bytes) => bytes,
        Err(ModelFailure::Missing) => return Ok(None),
        Err(other) => return Err(format!("{path} cannot be read: {other}")),
    };
    let logic = RigLogic::read(&bytes).map_err(|error| format!("{path}: {error}"))?;
    if logic.has_joint_deltas() {
        Ok(Some(logic))
    } else {
        Err(format!("{path} carries no joint delta records"))
    }
}

/// Returns the bones of the head `model` with the bind poses of the same-named bones of the
/// skeleton file `<skin>_skel.chr` beside its skin at `skin`; `None` when the Archive has no such
/// file.
///
/// The skin's own copy of the facial bones is the stock head's, symmetric; the skeleton file has
/// the joints where the character's face puts them.
fn posed_bones(
    model: &Model,
    skin: &str,
    models: &dyn ModelProvider,
) -> Result<Option<Vec<Bone>>, String> {
    let Some((stem, _)) = skin.rsplit_once('.') else {
        return Ok(None);
    };
    let path = format!("{stem}_skel.chr");
    let file = match models.model(&path, 0) {
        Ok(provided) => provided.model,
        Err(ModelFailure::Missing) => return Ok(None),
        Err(other) => return Err(format!("{path} cannot be read: {other}")),
    };
    let own: HashMap<String, &Bone> = bones_of(&file)
        .iter()
        .map(|bone| (bone.name.to_ascii_lowercase(), bone))
        .collect();
    let posed = bones_of(model)
        .iter()
        .map(|bone| {
            let mut bone = bone.clone();
            if let Some(found) = own.get(&bone.name.to_ascii_lowercase()) {
                bone.relative = found.relative;
            }
            bone
        })
        .collect();
    Ok(Some(posed))
}

/// Makes the face rig of the head from the `.RigLogic` file at `path`.
pub(super) fn own_rig(path: &str, models: &dyn ModelProvider) -> Result<FaceRig, String> {
    let logic = read_logic(path, models)?.ok_or_else(|| format!("{path} does not exist"))?;
    FaceRig::from_logic(&logic).map_err(|error| format!("{path}: {error}"))
}

/// Reads the blendshapes of the head whose skin is at `skin` from the `BShapesGpu` chunk of its
/// geometry file and makes the morph targets `expressions` asks for of its `mesh`.
fn morphs(
    model: &Model,
    skin: &str,
    (logic, expressions): (&RigLogic, &Expressions),
    models: &dyn ModelProvider,
) -> Result<Morphs, String> {
    let [mesh] = model
        .lods
        .first()
        .map_or(&[][..], |lod| lod.meshes.as_slice())
    else {
        return Err("a head has exactly one mesh".to_owned());
    };
    let geometry = format!("{skin}m");
    let bytes = models
        .file(&geometry, MAX_FILE_LEN)
        .map_err(|failure| format!("{geometry} cannot be read: {failure}"))?;
    let chunks =
        ChunkFile::parse(bytes.as_slice()).map_err(|error| format!("{geometry}: {error}"))?;
    let (index, _) = chunks
        .chunks_of(ivo::BSHAPES_GPU)
        .next()
        .ok_or_else(|| format!("{geometry} has no BShapesGpu chunk"))?;
    let data = chunks
        .chunk_data(index)
        .ok_or_else(|| format!("{geometry}: the BShapesGpu chunk lies outside the file"))?;
    let shapes = SkinBlendshapes::read(data).map_err(|error| format!("{geometry}: {error}"))?;
    if shapes.vertices() != mesh.positions.len() {
        return Err(format!(
            "{geometry} covers {} vertices, the skin has {}",
            shapes.vertices(),
            mesh.positions.len()
        ));
    }
    if shapes.blendshapes() != logic.blendshapes() {
        return Err(format!(
            "{geometry} has {} blendshapes, the rig logic {}",
            shapes.blendshapes(),
            logic.blendshapes()
        ));
    }
    Ok(targets_of(&shapes, logic, mesh, expressions))
}

/// Returns what the own rig of the head `model`, whose skin is at `skin`, contributes: the joints
/// of its face from the `.RigLogic` file, posed by the skeleton file beside the skin, and, for
/// `expressions`, the blendshapes of its geometry as morph targets. `None` for a head without a `.RigLogic` file and for a level of detail
/// below 0, which has no rig.
pub(super) fn own_head(
    model: &Model,
    skin: &str,
    expressions: &Expressions,
    models: &dyn ModelProvider,
) -> Option<Outcome> {
    if model.lods.first()?.level != 0 {
        return None;
    }
    let path = logic_path(skin)?;
    let logic = match read_logic(&path, models) {
        Ok(found) => found?,
        Err(reason) => return Some(outcome(None, Some(Err(reason)))),
    };
    let made = (*expressions != Expressions::None)
        .then(|| morphs(model, skin, (&logic, expressions), models));
    let face = posed_bones(model, skin, models)
        .map(|posed| (own_facial(model, &logic, posed), path.clone()));
    Some(outcome(made, Some(face)))
}
