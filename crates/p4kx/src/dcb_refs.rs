// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{DataForge, DataForgeEntry, IndexedReference, ReferenceIndex};
use thiserror::Error;

use crate::cli::{DcbRefsArgs, RefsFormat};
use crate::{dcb_refs_json, dcb_refs_table, exit};

/// The text names no Record.
#[derive(Debug, Error)]
#[error("no Record has the GUID or record path `{0}`")]
pub(crate) struct UnknownRecord(String);

/// `--json` with `--format table`.
#[derive(Debug, Error)]
#[error("--json writes JSON: leave --format out or use --format json")]
pub(crate) struct JsonWithTable;

/// What `p4kx dcb refs` lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Incoming,
    Outgoing,
    Files,
    Unresolved,
}

/// A Record as the listings name it.
#[derive(Debug, Clone)]
pub(crate) struct Info<'f> {
    pub(crate) guid: String,
    pub(crate) record_path: &'f str,
    pub(crate) name: &'f str,
    pub(crate) type_name: &'f str,
}

/// One Reference of a listing.
#[derive(Debug, Clone)]
pub(crate) struct Link<'f> {
    pub(crate) record: Option<Info<'f>>,
    pub(crate) guid: Option<String>,
    pub(crate) location: String,
    pub(crate) field: String,
    pub(crate) indices: Vec<u32>,
}

/// The References to or from one Record.
#[derive(Debug)]
pub(crate) struct Listing<'f> {
    pub(crate) target: Info<'f>,
    pub(crate) references: Vec<Link<'f>>,
    pub(crate) unresolved: Vec<Link<'f>>,
}

/// The edges between record files, as pairs of record paths.
#[derive(Debug)]
pub(crate) struct Edges<'f> {
    pub(crate) files: usize,
    pub(crate) edges: Vec<(&'f str, &'f str)>,
}

impl DcbRefsArgs {
    /// Returns what the arguments ask for.
    const fn mode(&self) -> Mode {
        if self.incoming {
            Mode::Incoming
        } else if self.outgoing {
            Mode::Outgoing
        } else if self.files {
            Mode::Files
        } else {
            Mode::Unresolved
        }
    }
}

/// Lists the References of the Archive's DataForge as the arguments ask and writes the listing
/// to stdout.
pub(crate) fn run(args: &DcbRefsArgs, as_json: bool) -> Result<ExitCode> {
    let json = match (as_json, args.format) {
        (true, Some(RefsFormat::Table)) => return Err(JsonWithTable.into()),
        (as_json, format) => as_json || format == Some(RefsFormat::Json),
    };
    let archive = crate::key::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let targets = match args.target.as_deref() {
        Some(text) => resolve(&forge, text)?,
        None => Vec::new(),
    };
    let index = ReferenceIndex::build(&forge);
    warn(&index);
    let mut out = BufWriter::new(io::stdout().lock());
    let written = match args.mode() {
        mode @ (Mode::Incoming | Mode::Outgoing) => {
            let listings = listings(&forge, &index, &targets, mode);
            if json {
                dcb_refs_json::listings(&mut out, &args.archive, mode, &listings)
            } else {
                dcb_refs_table::listings(&mut out, mode, &listings)
            }
        }
        Mode::Files => {
            let edges = edges(&forge, &index, &targets);
            if json {
                dcb_refs_json::files(&mut out, &args.archive, &edges)
            } else {
                dcb_refs_table::files(&mut out, &edges)
            }
        }
        Mode::Unresolved => {
            let links = unresolved(&forge, &index);
            if json {
                dcb_refs_json::unresolved(&mut out, &args.archive, &links)
            } else {
                dcb_refs_table::unresolved(&mut out, &links)
            }
        }
    };
    match written.and_then(|()| out.flush()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(ExitCode::from(exit::SUCCESS)),
    }
}

/// Returns the indices of the Records `text` names: the one with that GUID, or those at that
/// record path.
fn resolve(forge: &DataForge<'_>, text: &str) -> Result<Vec<u32>, UnknownRecord> {
    let found = forge.records_named(text);
    if found.is_empty() {
        return Err(UnknownRecord(text.to_owned()));
    }
    Ok(found)
}

fn warn(index: &ReferenceIndex) {
    let mut err = io::stderr().lock();
    if index.is_truncated() {
        let _unreported = writeln!(
            err,
            "warning: the DataForge holds more than {} References; the index is incomplete",
            p4kx_extraction::MAX_INDEXED_REFERENCES
        );
    }
    if index.unreadable_records() > 0 {
        let _unreported = writeln!(
            err,
            "warning: {} Records cannot be read and are not indexed",
            index.unreadable_records()
        );
    }
}

fn info<'f>(forge: &'f DataForge<'f>, record: u32) -> Option<Info<'f>> {
    let record = forge.record(record)?;
    Some(Info {
        guid: record.guid().to_string(),
        record_path: record.record_path().unwrap_or_default(),
        name: record.name().unwrap_or_default(),
        type_name: record.type_name().unwrap_or_default(),
    })
}

fn link<'f>(
    forge: &'f DataForge<'f>,
    reference: &IndexedReference<'_>,
    other: Option<u32>,
) -> Link<'f> {
    Link {
        record: other.and_then(|record| info(forge, record)),
        guid: reference.unresolved_guid().map(|guid| guid.to_string()),
        location: reference.location(),
        field: reference.field_path().to_owned(),
        indices: reference.indices().to_vec(),
    }
}

fn listings<'f>(
    forge: &'f DataForge<'f>,
    index: &ReferenceIndex,
    targets: &[u32],
    mode: Mode,
) -> Vec<Listing<'f>> {
    targets
        .iter()
        .filter_map(|&target| {
            let mut listing = Listing {
                target: info(forge, target)?,
                references: Vec::new(),
                unresolved: Vec::new(),
            };
            if mode == Mode::Incoming {
                for reference in index.incoming(target) {
                    listing
                        .references
                        .push(link(forge, &reference, Some(reference.source())));
                }
            } else {
                for reference in index.outgoing(target) {
                    match reference.target() {
                        Some(other) => {
                            listing
                                .references
                                .push(link(forge, &reference, Some(other)));
                        }
                        None => listing.unresolved.push(link(forge, &reference, None)),
                    }
                }
            }
            Some(listing)
        })
        .collect()
}

fn unresolved<'f>(forge: &'f DataForge<'f>, index: &ReferenceIndex) -> Vec<Link<'f>> {
    index
        .unresolved()
        .map(|reference| link(forge, &reference, Some(reference.source())))
        .collect()
}

/// Returns the edges of the file graph; with `targets`, only those from or to their files.
fn edges<'f>(forge: &'f DataForge<'f>, index: &ReferenceIndex, targets: &[u32]) -> Edges<'f> {
    let graph = index.file_graph(forge);
    let wanted: Vec<u32> = targets
        .iter()
        .filter_map(|&target| {
            let path = forge.record(target)?.record_path().ok()?;
            graph.position(path)
        })
        .collect();
    let paths = graph.paths();
    let edges = graph
        .edges()
        .iter()
        .filter(|(from, to)| targets.is_empty() || wanted.contains(from) || wanted.contains(to))
        .filter_map(|(from, to)| {
            let path = |file: &u32| paths.get(*file as usize).copied();
            Some((path(from)?, path(to)?))
        })
        .collect();
    Edges {
        files: paths.len(),
        edges,
    }
}
