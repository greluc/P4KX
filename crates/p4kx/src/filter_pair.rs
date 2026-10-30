// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The selection of Entries by an include Filter and exclusion Filters.

use anyhow::Result;
use p4kx_extraction::{Archive, Entry, Exclusion, Filter, TextureGroups};
use thiserror::Error;

use crate::no_match::NoMatch;

/// An include Filter whose every match an exclusion Filter leaves out.
#[derive(Debug, Error)]
#[error("the exclusion Filters leave out all {excluded} Entries that {include} matches")]
pub(crate) struct ExcludedAll {
    include: String,
    excluded: usize,
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

fn narrow<'a>(
    found: Vec<&'a Entry>,
    text: &str,
    exclusion: &Exclusion,
    required: bool,
) -> Result<Selected<'a>> {
    if found.is_empty() && required {
        return Err(NoMatch::new(text).into());
    }
    let left = exclusion.leave_out(found, |entry: &&'a Entry| -> &'a str { entry.path() });
    if left.kept.is_empty() && required {
        return Err(ExcludedAll {
            include: text.to_owned(),
            excluded: left.removed,
        }
        .into());
    }
    Ok(Selected {
        entries: left.kept,
        excluded_parts: left.parts_of_kept_textures,
    })
}
