// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{
    Archive, AudioConversion, Entry, TextureConversion, UnknownEntries, select_entries,
};
use serde::Serialize;

use crate::cli::{SocpakAction, SocpakArgs, SocpakSource};
use crate::extract::{self, Settings};
use crate::json::{self, EntryJson};
use crate::{exit, list, verify};

#[derive(Serialize)]
struct ListJson<'a> {
    archive: String,
    containers: &'a [String],
    entries: Vec<EntryJson<'a>>,
}

/// Runs `p4kx socpak list` or `p4kx socpak extract`.
pub(crate) fn run(args: &SocpakArgs, as_json: bool) -> Result<ExitCode> {
    match &args.action {
        SocpakAction::List(source) => {
            let (container, containers) = open(source)?;
            let entries: Vec<&Entry> = container.entries().iter().collect();
            if as_json {
                json::print(&ListJson {
                    archive: json::path(container.path()),
                    containers: &containers,
                    entries: entries.iter().copied().map(EntryJson::from).collect(),
                })?;
            } else {
                list::print_table(&entries)?;
            }
            Ok(ExitCode::from(exit::SUCCESS))
        }
        SocpakAction::Verify { source, filter } => {
            let (container, _) = open(source)?;
            let entries = verify::selected(&container, filter.as_deref())?;
            verify::execute(&container, &entries, as_json)
        }
        SocpakAction::Extract {
            source,
            filter,
            out,
            conflict,
            convert,
            verify,
        } => {
            let (container, _) = open(source)?;
            let entries = match filter {
                Some(text) => extract::filtered_entries(&container, text)?,
                None => container.entries().iter().collect(),
            };
            let settings = Settings {
                out: out.as_deref(),
                conflict: *conflict,
                convert: *convert,
                textures: TextureConversion {
                    merge: true,
                    png: Some(0),
                    keep_raw: false,
                },
                audio: AudioConversion::Ogg,
                verify: *verify,
                threads: None,
                cancel_after: None,
            };
            extract::execute(&container, &entries, &settings, as_json)
        }
    }
}

/// Opens the Archive and every container of the chain in turn; returns the last container and
/// the Entry paths of the chain as the Archive and the containers store them.
fn open(source: &SocpakSource) -> Result<(Archive, Vec<String>)> {
    let mut current = crate::key::open(&source.archive)?;
    let mut chain = Vec::with_capacity(source.containers.len());
    for path in &source.containers {
        let found = select_entries(&current, &[path])?;
        let Some(entry) = found.first() else {
            return Err(UnknownEntries(vec![path.clone()]).into());
        };
        chain.push(entry.path().to_owned());
        current = current.open_container(entry)?;
    }
    Ok((current, chain))
}
