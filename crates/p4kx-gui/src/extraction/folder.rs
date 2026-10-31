// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use p4kx_extraction::{
    CancelToken, ConflictPolicy, ExtractOptions, FolderError, FolderSource, Progress,
    convert_folder,
};
use p4kx_settings::{ExtractionAudio, ExtractionTextures};

use super::job::{Origin, RunResult, launch};
use super::{ExtractionSummary, audio_conversion, texture_conversion};
use crate::Extraction;
use crate::model::describe;

/// What the conversion of a folder reads, writes and converts to.
#[derive(Debug, Clone)]
pub(crate) struct FolderRequest {
    pub(crate) input: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) textures: ExtractionTextures,
    pub(crate) audio: ExtractionAudio,
}

/// Starts converting the files of the folder of `request` on a background thread, calling `wake`
/// from it once it ended. Files that exist in the output are kept; files without a conversion are
/// left out.
pub(crate) fn start_folder(request: FolderRequest, wake: impl Fn() + Send + 'static) -> Extraction {
    let origin = Origin {
        archive: request.input.clone(),
        folder: request.input.clone(),
        comparison_side: false,
        folder_conversion: true,
        entries_total: 0,
    };
    launch(origin, wake, move |cancel, progress| {
        run(&request, cancel, progress)
    })
}

fn run(request: &FolderRequest, cancel: &CancelToken, progress: &Mutex<Progress>) -> RunResult {
    let source = match FolderSource::scan(&request.input, Some(cancel)) {
        Ok(source) => source,
        Err(FolderError::Cancelled) => {
            return Ok(ExtractionSummary::cancelled_scan(
                request.output.clone(),
                &request.input,
            ));
        }
        Err(error) => return Err(describe(&error)),
    };
    let on_progress = |now: Progress| {
        *progress.lock().unwrap_or_else(PoisonError::into_inner) = now;
    };
    on_progress(Progress {
        entries_total: source.len(),
        bytes_total: source.total_size(),
        ..Progress::default()
    });
    let options = ExtractOptions {
        policy: ConflictPolicy::Skip,
        convert: true,
        textures: texture_conversion(request.textures),
        audio: audio_conversion(request.audio),
        cancel: Some(cancel),
        progress: Some(&on_progress),
        ..ExtractOptions::default()
    };
    let report = convert_folder(&source, None, false, &request.output, &options)
        .map_err(|error| describe(&error))?;
    Ok(ExtractionSummary::new(request.output.clone(), &report))
}
