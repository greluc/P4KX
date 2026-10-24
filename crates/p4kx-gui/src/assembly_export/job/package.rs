// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{ArchiveModels, Assembly, LoadoutChanges};
use p4kx_blender::{BuildInfo, PackageError, PackageInfo, Replacement, write_package};
use p4kx_channels::channel_name;
use p4kx_extraction::{Archive, CancelToken};

use super::run::failed;
use super::stop::{Stoppable, StoppableTextures};
use super::{Ended, Request};
use crate::assembly_export::AssemblyExportSummary;
use crate::{EntryIndex, ModelExportFailure};

/// Writes `assembly` as the Blender package of `request`, replacing an earlier package of the
/// same folder only once the new one is complete.
pub(super) fn write(
    index: &EntryIndex,
    request: &Request,
    (assembly, models): (&Assembly, &ArchiveModels<'_>),
    (changes, cancel): (&LoadoutChanges, &CancelToken),
) -> Ended {
    let provider = Stoppable { models, cancel };
    let loader = models.textures();
    let textures = StoppableTextures {
        textures: &loader,
        cancel,
    };
    let info = info_of(index.archive(), changes);
    let written = write_package(
        assembly,
        &provider,
        &textures,
        &request.options(),
        &info,
        &request.file,
        &|| cancel.is_cancelled(),
    );
    match written {
        Ok(summary) => Ended::Written(Box::new(AssemblyExportSummary::of_package(
            (request.file.clone(), request.name.clone()),
            assembly,
            (summary, changes),
        ))),
        Err(PackageError::Cancelled) => Ended::Cancelled,
        Err(_) if cancel.is_cancelled() => Ended::Cancelled,
        Err(PackageError::Refused(refused)) => Ended::Failed(ModelExportFailure::Refused(refused)),
        Err(error) => failed(index, &error),
    }
}

fn info_of(archive: &Archive, changes: &LoadoutChanges) -> PackageInfo {
    let build = archive.build().map(|build| BuildInfo {
        branch: build.branch().to_owned(),
        version: build.version().to_owned(),
        public_patch: build.public_patch().map(str::to_owned),
        change_number: build.change_number().to_owned(),
        build_id: build.build_id().to_owned(),
    });
    let replace = changes
        .replacements()
        .iter()
        .map(|(port, entity)| Replacement {
            port: port.clone(),
            entity: entity.clone(),
        })
        .collect();
    PackageInfo::new(channel_name(archive.path()), build)
        .with_changes(changes.exclusions().to_vec(), replace)
}
