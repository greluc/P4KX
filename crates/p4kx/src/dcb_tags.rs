// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{DataForgeEntry, Guid, Tag, TagTree, TagTreeIssues};
use thiserror::Error;

use crate::cli::{DcbTagsArgs, TagsFormat};
use crate::dcb_refs::JsonWithTable;
use crate::{dcb_tags_json, dcb_tags_table, exit};

/// The text names no tag.
#[derive(Debug, Error)]
#[error("no tag has the tag path or GUID `{0}`")]
pub(crate) struct UnknownTag(String);

/// One tag of a listing and how far it is indented below the first tag shown.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Row<'t> {
    pub(crate) tag: Tag<'t>,
    pub(crate) indent: usize,
}

/// The tags `p4kx dcb tags` shows: the tags asked for, each followed by everything below it.
#[derive(Debug)]
pub(crate) struct Listing<'t> {
    pub(crate) tops: Vec<Tag<'t>>,
    pub(crate) rows: Vec<Row<'t>>,
    pub(crate) total: usize,
    pub(crate) issues: TagTreeIssues,
}

/// Lists the tag tree of the Archive's DataForge as the arguments ask and writes it to stdout.
pub(crate) fn run(args: &DcbTagsArgs, as_json: bool) -> Result<ExitCode> {
    let json = match (as_json, args.format) {
        (true, Some(TagsFormat::Table)) => return Err(JsonWithTable.into()),
        (as_json, format) => as_json || format == Some(TagsFormat::Json),
    };
    let archive = crate::key::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let tags = TagTree::build(&forge);
    warn(tags.issues());
    let listing = listing(&tags, args.tag.as_deref())?;
    let mut out = BufWriter::new(io::stdout().lock());
    let written = if json {
        dcb_tags_json::write(&mut out, &args.archive, &listing, args.tree)
    } else {
        dcb_tags_table::write(&mut out, &listing, args.tree)
    };
    match written.and_then(|()| out.flush()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(ExitCode::from(exit::SUCCESS)),
    }
}

/// Returns the tags `text` names, each with everything below it, or every tag without `text`.
fn listing<'t>(tags: &'t TagTree, text: Option<&str>) -> Result<Listing<'t>, UnknownTag> {
    let tops: Vec<Tag<'t>> = match text {
        None => tags.roots().collect(),
        Some(text) => {
            let found: Vec<Tag<'t>> = text.parse::<Guid>().map_or_else(
                |_| tags.find_path(text).collect(),
                |guid| tags.get(guid).into_iter().collect(),
            );
            if found.is_empty() {
                return Err(UnknownTag(text.to_owned()));
            }
            found
        }
    };
    let rows = tops
        .iter()
        .flat_map(|top| {
            let first = top.depth();
            std::iter::once(*top)
                .chain(top.descendants())
                .map(move |tag| Row {
                    tag,
                    indent: tag.depth().saturating_sub(first),
                })
        })
        .collect();
    Ok(Listing {
        tops,
        rows,
        total: tags.len(),
        issues: tags.issues(),
    })
}

fn warn(issues: TagTreeIssues) {
    let mut err = io::stderr().lock();
    let notes = [
        (
            issues.unreadable(),
            "Tag Records cannot be read and are left out",
        ),
        (issues.dangling(), "listed tags are no Tag Record"),
        (
            issues.repeated(),
            "tags are listed more than once; the first listing counts",
        ),
        (issues.too_deep(), "tags are too deep and left out"),
        (
            issues.too_long(),
            "tags have a name or path that is too long and are left out",
        ),
        (
            issues.duplicate(),
            "Tag Records repeat the GUID of an earlier one and are left out",
        ),
        (
            issues.unlisted(),
            "tags are not listed by the TagDatabase and stand as roots of their own",
        ),
    ];
    for (count, note) in notes {
        if count > 0 {
            let _unreported = writeln!(err, "warning: {count} {note}");
        }
    }
}
