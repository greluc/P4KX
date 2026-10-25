// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{ArchiveModels, CharacterAnimations, LoadoutChanges};
use p4kx_blender::{CharacterScene, write_character_package};
use p4kx_extraction::{CancelToken, GlbSummary};

use super::export::{CharacterExportSummary, Ended, Request};
use crate::assembly_export::{PartIssue, StoppableTextures, Verdict, failure_of, info_of, verdict};
use crate::{EntryIndex, ModelExportFailure};

/// Writes the Character of `request` as a Blender package, replacing an earlier package of the
/// same folder only once the new one is complete.
///
/// The package carries the skeleton and the shape keys of the Character as built; the tab offers
/// no animation clips, so it has none.
pub(super) fn write(index: &EntryIndex, request: &Request, cancel: &CancelToken) -> Ended {
    let models = ArchiveModels::new(index.archive());
    let archive_textures = models.textures();
    let textures = StoppableTextures {
        textures: &archive_textures,
        cancel,
    };
    let animations = CharacterAnimations::default();
    let scene = CharacterScene::new(&request.name, &request.armour, &animations).with_origin(
        request.origin.chf.as_deref(),
        request.origin.loadout.as_deref(),
    );
    let info = info_of(index.archive(), &LoadoutChanges::default());
    let written = write_character_package(
        &scene,
        &textures,
        &request.options(),
        &info,
        &request.file,
        &|| cancel.is_cancelled(),
    );
    match written {
        Ok(package) => Ended::Written(Box::new(CharacterExportSummary {
            file: request.file.clone(),
            name: request.name.clone(),
            parts: request.armour.parts().len(),
            lod: request.armour.lod(),
            written: GlbSummary::default(),
            issues: package.issues.iter().map(PartIssue::of_package).collect(),
            package: Some(package),
        })),
        Err(error) => match verdict(error, cancel.is_cancelled()) {
            Verdict::Cancelled => Ended::Cancelled,
            Verdict::Refused(refused) => Ended::Failed(ModelExportFailure::Refused(refused)),
            Verdict::Failed(error) => Ended::Failed(failure_of(index, &error)),
        },
    }
}
