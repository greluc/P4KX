// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

use serde::Serialize;

use crate::dcb_refs::{Edges, Info, Link, Listing, Mode};
use crate::json;

#[derive(Serialize)]
struct InfoJson<'a> {
    guid: &'a str,
    record_path: &'a str,
    name: &'a str,
    #[serde(rename = "type")]
    type_name: &'a str,
}

impl<'a> From<&'a Info<'_>> for InfoJson<'a> {
    fn from(info: &'a Info<'_>) -> Self {
        Self {
            guid: &info.guid,
            record_path: info.record_path,
            name: info.name,
            type_name: info.type_name,
        }
    }
}

#[derive(Serialize)]
struct LinkJson<'a> {
    record: Option<InfoJson<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guid: Option<&'a str>,
    location: &'a str,
    field: &'a str,
    indices: &'a [u32],
}

impl<'a> From<&'a Link<'_>> for LinkJson<'a> {
    fn from(link: &'a Link<'_>) -> Self {
        Self {
            record: link.record.as_ref().map(InfoJson::from),
            guid: link.guid.as_deref(),
            location: &link.location,
            field: &link.field,
            indices: &link.indices,
        }
    }
}

#[derive(Serialize)]
struct ListingJson<'a> {
    record: InfoJson<'a>,
    reference_count: usize,
    record_count: usize,
    references: Vec<LinkJson<'a>>,
    unresolved_count: usize,
    unresolved: Vec<LinkJson<'a>>,
}

#[derive(Serialize)]
struct ListingsJson<'a> {
    archive: String,
    direction: &'static str,
    records: Vec<ListingJson<'a>>,
}

/// Writes the listings of `mode` as one line of JSON.
pub(crate) fn listings(
    out: &mut impl Write,
    archive: &Path,
    mode: Mode,
    listings: &[Listing<'_>],
) -> io::Result<()> {
    let records = listings
        .iter()
        .map(|listing| ListingJson {
            record: InfoJson::from(&listing.target),
            reference_count: listing.references.len(),
            record_count: listing
                .references
                .iter()
                .filter_map(|link| link.record.as_ref())
                .map(|info| info.guid.as_str())
                .collect::<HashSet<_>>()
                .len(),
            references: listing.references.iter().map(LinkJson::from).collect(),
            unresolved_count: listing.unresolved.len(),
            unresolved: listing.unresolved.iter().map(LinkJson::from).collect(),
        })
        .collect();
    line(
        out,
        &ListingsJson {
            archive: json::path(archive),
            direction: if mode == Mode::Incoming {
                "incoming"
            } else {
                "outgoing"
            },
            records,
        },
    )
}

#[derive(Serialize)]
struct EdgeJson<'a> {
    from: &'a str,
    to: &'a str,
}

#[derive(Serialize)]
struct FilesJson<'a> {
    archive: String,
    file_count: usize,
    edge_count: usize,
    edges: Vec<EdgeJson<'a>>,
}

/// Writes the edges of the file graph as one line of JSON.
pub(crate) fn files(out: &mut impl Write, archive: &Path, edges: &Edges<'_>) -> io::Result<()> {
    line(
        out,
        &FilesJson {
            archive: json::path(archive),
            file_count: edges.files,
            edge_count: edges.edges.len(),
            edges: edges
                .edges
                .iter()
                .map(|(from, to)| EdgeJson { from, to })
                .collect(),
        },
    )
}

#[derive(Serialize)]
struct UnresolvedJson<'a> {
    archive: String,
    count: usize,
    unresolved: Vec<LinkJson<'a>>,
}

/// Writes every unresolved Reference as one line of JSON.
pub(crate) fn unresolved(
    out: &mut impl Write,
    archive: &Path,
    links: &[Link<'_>],
) -> io::Result<()> {
    line(
        out,
        &UnresolvedJson {
            archive: json::path(archive),
            count: links.len(),
            unresolved: links.iter().map(LinkJson::from).collect(),
        },
    )
}

fn line(out: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)
}
