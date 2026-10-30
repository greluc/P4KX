// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use super::{
    ArchiveModels, ModelFailure, ModelProvider, ProvidedAttachment, ProvidedModel, without_mtl,
};
use crate::build_models::vertices;
use crate::cdf::{self, MAX_CDF_BYTES};
use crate::cdf_bone::BoneAttachment;

impl ArchiveModels<'_> {
    /// Reads the character definition at `entry`: its skin as the model, its bone attachments as
    /// models of their own, and the files of what it cannot draw.
    pub(super) fn character(&self, entry: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        let bytes = self.file(entry, MAX_CDF_BYTES)?;
        let unreadable = |reason: String| ModelFailure::Unreadable {
            reason: format!("{entry}: {reason}"),
        };
        let mut character = cdf::read(&bytes).map_err(unreadable)?;
        let model_entry = self
            .entry_of(&character.model)
            .ok_or_else(|| unreadable(format!("{} does not exist", character.model)))?;
        let mut provided = self.read_model(model_entry, lod, Vec::new())?;
        let mut remaining = self
            .max_attachment_vertices
            .saturating_sub(vertices(&provided.model));
        for bone in &character.bones {
            if remaining == 0 {
                character.left_out.push(bone.binding.clone());
                continue;
            }
            match self.attachment(bone, lod) {
                Ok(attachment) if vertices(&attachment.model) <= remaining => {
                    remaining -= vertices(&attachment.model);
                    provided.attachments.push(attachment);
                }
                Ok(_) => {
                    remaining = 0;
                    character.left_out.push(bone.binding.clone());
                }
                Err(ModelFailure::Fatal(source)) => return Err(ModelFailure::Fatal(source)),
                Err(_) => character.left_out.push(bone.binding.clone()),
            }
        }
        provided.left_out = character.left_out;
        provided.skeleton = character
            .skeleton
            .as_deref()
            .map_or(Ok(None), |path| self.skeleton_problem(path))?;
        if let Some(material) = &character.material {
            provided.model.material_library = Some(without_mtl(material).to_owned());
        }
        Ok(provided)
    }

    fn attachment(
        &self,
        bone: &BoneAttachment,
        lod: u32,
    ) -> Result<ProvidedAttachment, ModelFailure> {
        let entry = self.entry_of(&bone.binding).ok_or(ModelFailure::Missing)?;
        let ProvidedModel {
            entry, mut model, ..
        } = self.read_model(entry, lod, Vec::new())?;
        if let Some(material) = &bone.material {
            model.material_library = Some(without_mtl(material).to_owned());
        }
        Ok(ProvidedAttachment {
            name: bone.name.clone(),
            bone: bone.bone.clone(),
            position: bone.position,
            rotation: bone.rotation,
            entry,
            model: Arc::new(model),
        })
    }
}
