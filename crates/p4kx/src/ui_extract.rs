// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{UiExtraction, UiMovie};
use serde::Serialize;

use crate::ui::chain;
use crate::{exit, json};

#[derive(Serialize)]
struct ExtractJson<'a> {
    movie: &'a str,
    target: String,
    extracted: Vec<WrittenJson>,
    failed: Vec<FailedJson>,
}

#[derive(Serialize)]
struct WrittenJson {
    kind: &'static str,
    id: u16,
    file: String,
    size: u64,
}

#[derive(Serialize)]
struct FailedJson {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<usize>,
    error: String,
}

/// Runs `p4kx ui extract` on an opened movie.
pub(crate) fn extract(movie: &UiMovie<'_>, out: &Path, as_json: bool) -> Result<ExitCode> {
    let extraction = movie.extract_assets(out)?;
    let failed = failed(&extraction);
    if as_json {
        json::print(&ExtractJson {
            movie: movie.name(),
            target: json::path(out),
            extracted: extraction
                .written
                .iter()
                .map(|written| WrittenJson {
                    kind: written.kind.name(),
                    id: written.id,
                    file: json::path(&written.path),
                    size: written.size,
                })
                .collect(),
            failed,
        })?;
    } else {
        let mut stderr = io::stderr().lock();
        for failure in &failed {
            match (failure.code, failure.offset) {
                (Some(code), Some(offset)) => writeln!(
                    stderr,
                    "error: unreadable tag {code} at offset {offset}: {}",
                    failure.error
                )?,
                _ => writeln!(stderr, "error: {}", failure.error)?,
            }
        }
        writeln!(
            io::stdout().lock(),
            "extracted {} of {} definitions from {} into {}; {} tags unreadable",
            extraction.written.len(),
            extraction.written.len() + extraction.failures.len(),
            movie.name(),
            out.display(),
            extraction.tag_failures.len()
        )?;
    }
    Ok(ExitCode::from(if extraction.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn failed(extraction: &UiExtraction) -> Vec<FailedJson> {
    let tags = extraction.tag_failures.iter().map(|failure| FailedJson {
        kind: "tag",
        id: None,
        code: Some(failure.code),
        offset: Some(failure.offset),
        error: chain(&failure.error),
    });
    let definitions = extraction.failures.iter().map(|failure| FailedJson {
        kind: failure.kind.name(),
        id: Some(failure.id),
        code: None,
        offset: None,
        error: chain(&failure.error),
    });
    tags.chain(definitions).collect()
}
