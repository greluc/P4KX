// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The selection of Entries by an include Filter and exclusion Filters.

use std::io::{self, Write};

use anyhow::Result;
use p4kx_extraction::{Archive, Entry, Exclusion, Filter, TextureGroups};
use thiserror::Error;

use crate::no_match::NoMatch;

/// Notes on stderr the `count` part Entries of textures that stay which an exclusion Filter left
/// out and which `reader`, the option that converts, still reads; says nothing for none.
pub(crate) fn note_excluded_parts(count: usize, reader: &str) {
    if count > 0 {
        let _ignored = writeln!(
            io::stderr().lock(),
            "note: --exclude left out {count} part Entries of textures that stay; {reader} still \
             reads them to build each texture"
        );
    }
}

/// An include Filter whose every match an exclusion Filter leaves out.
#[derive(Debug, Error)]
#[error("the exclusion Filters leave out all {excluded} Entries that {include} matches")]
pub(crate) struct ExcludedAll {
    include: String,
    excluded: usize,
}

impl ExcludedAll {
    /// Creates the error for the include Filter `include` whose `excluded` matches all left out.
    pub(crate) fn new(include: &str, excluded: usize) -> Self {
        Self {
            include: include.to_owned(),
            excluded,
        }
    }
}

/// Refuses a selection of `kept` of `matched` files or Entries that is empty.
///
/// # Errors
///
/// Returns [`NoMatch`] when the Filter `text` matched none and [`ExcludedAll`] when the exclusion
/// Filters left out all it matched.
pub(crate) fn require(text: &str, matched: usize, kept: usize) -> Result<()> {
    if matched == 0 {
        Err(NoMatch::new(text).into())
    } else if kept == 0 {
        Err(ExcludedAll::new(text, matched).into())
    } else {
        Ok(())
    }
}

/// Returns the Entries of `archive` that the Filter `text` matches without those `exclude`
/// matches, as [`extraction`] does, and with `convert` notes on stderr the excluded parts of
/// textures that stay, which the Conversion reads all the same.
///
/// # Errors
///
/// As [`extraction`].
pub(crate) fn extracting<'a>(
    archive: &'a Archive,
    text: &str,
    exclude: &[String],
    convert: bool,
) -> Result<Vec<&'a Entry>> {
    let selected = extraction(archive, text, exclude)?;
    if convert {
        note_excluded_parts(selected.excluded_parts.len(), "--convert");
    }
    Ok(selected.entries)
}

/// The Entries a Filter pair selects.
pub(crate) struct Selected<'a> {
    /// The selected Entries in Archive order.
    pub(crate) entries: Vec<&'a Entry>,
    /// The part Entries of kept textures that an exclusion Filter matched.
    pub(crate) excluded_parts: Vec<&'a Entry>,
}

/// Returns the Entries of `archive` that the Filter `text` matches without an exclusion Filter
/// `exclude` matching them, and, for a texture one of them belongs to, all its Entries not left
/// out.
///
/// # Errors
///
/// Returns an error for invalid Filter text, [`NoMatch`] when `text` matches none and
/// [`ExcludedAll`] when the exclusion Filters leave out all it matches.
pub(crate) fn extraction<'a>(
    archive: &'a Archive,
    text: &str,
    exclude: &[String],
) -> Result<Selected<'a>> {
    let filter = Filter::parse(text)?;
    let exclusion = Exclusion::parse(exclude)?;
    let groups = TextureGroups::build(archive);
    narrow(
        filter.select_with_parts(archive, &groups),
        text,
        &exclusion,
        true,
    )
}

/// Returns the Entries of `archive` that the Filter `text` matches without an exclusion Filter
/// `exclude` matching them.
///
/// Without exclusion Filters an empty result is a result; with them it is an error, as for
/// [`extraction`].
///
/// # Errors
///
/// Returns an error for invalid Filter text, and with exclusion Filters [`NoMatch`] and
/// [`ExcludedAll`] as [`extraction`] does.
pub(crate) fn search<'a>(
    archive: &'a Archive,
    text: &str,
    exclude: &[String],
) -> Result<Selected<'a>> {
    let filter = Filter::parse(text)?;
    let exclusion = Exclusion::parse(exclude)?;
    narrow(
        filter.select(archive),
        text,
        &exclusion,
        !exclusion.is_empty(),
    )
}

/// Returns the Entries of `archive` to verify: those the Filter `text` matches, or every Entry
/// without a Filter, without those an exclusion Filter of `exclude` matches.
///
/// # Errors
///
/// Returns an error for invalid Filter text, [`NoMatch`] when `text` matches none and
/// [`ExcludedAll`] when the exclusion Filters leave out everything selected; the message names
/// `*` as the include Filter when there is none.
pub(crate) fn verification<'a>(
    archive: &'a Archive,
    text: Option<&str>,
    exclude: &[String],
) -> Result<Vec<&'a Entry>> {
    let exclusion = Exclusion::parse(exclude)?;
    let found = match text {
        Some(text) => Filter::parse(text)?.select(archive),
        None => archive.entries().iter().collect(),
    };
    let required = text.is_some() || !exclusion.is_empty();
    Ok(narrow(found, text.unwrap_or("*"), &exclusion, required)?.entries)
}

fn narrow<'a>(
    found: Vec<&'a Entry>,
    text: &str,
    exclusion: &Exclusion,
    required: bool,
) -> Result<Selected<'a>> {
    let matched = found.len();
    let left = exclusion.leave_out(found, |entry: &&'a Entry| -> &'a str { entry.path() });
    if required {
        require(text, matched, left.kept.len())?;
    }
    Ok(Selected {
        entries: left.kept,
        excluded_parts: left.parts_of_kept_textures,
    })
}
