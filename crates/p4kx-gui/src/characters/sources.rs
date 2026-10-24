// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::CancelToken;

use crate::EntryIndex;

const LOADOUTS: &str = "scripts/loadouts/";
const DATA: &str = "data/";

/// What the Archive offers to build a Character from: the CHF files it ships and its XML
/// Loadouts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources {
    pub(super) heads: Vec<String>,
    pub(super) loadouts: Vec<String>,
}

impl Sources {
    /// Returns the paths of the CHF Entries, below `Data`, sorted ignoring case.
    #[must_use]
    pub fn heads(&self) -> &[String] {
        &self.heads
    }

    /// Returns the paths of the XML Loadouts as a Record names them, sorted ignoring case.
    #[must_use]
    pub fn loadouts(&self) -> &[String] {
        &self.loadouts
    }
}

fn starts_with_ignoring_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

fn ends_with_ignoring_case(text: &str, suffix: &str) -> bool {
    text.len()
        .checked_sub(suffix.len())
        .and_then(|from| text.get(from..))
        .is_some_and(|end| end.eq_ignore_ascii_case(suffix))
}

/// Lists the CHF files and XML Loadouts of the Archive of `index`, or `None` once `cancel` is set.
pub(super) fn scan(index: &EntryIndex, cancel: &CancelToken) -> Option<Sources> {
    let mut sources = Sources::default();
    for (at, entry) in index.archive().entries().iter().enumerate() {
        if at % 4096 == 0 && cancel.is_cancelled() {
            return None;
        }
        let path = entry.path().replace('\\', "/");
        if ends_with_ignoring_case(&path, ".chf") {
            sources.heads.push(path);
        } else if ends_with_ignoring_case(&path, ".xml") {
            let record = if starts_with_ignoring_case(&path, DATA) {
                path.get(DATA.len()..).unwrap_or(&path)
            } else {
                &path
            };
            if starts_with_ignoring_case(record, LOADOUTS) {
                sources.loadouts.push(record.to_owned());
            }
        }
    }
    sources.heads.sort_by_cached_key(|path| path.to_ascii_lowercase());
    sources
        .loadouts
        .sort_by_cached_key(|path| path.to_ascii_lowercase());
    Some(sources)
}
