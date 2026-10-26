// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::path::Path;

use p4kx_archive::Build;

use crate::launcher_log::{self, Wanted};

const MARKER: &str = "(SC ";
const MAX_PATCHES: usize = 4096;
const MAX_VERSION_LEN: usize = 32;
const MAX_CHANGE_LEN: usize = 20;

/// The public patch versions the RSI Launcher logged, by Perforce change number.
///
/// Read once from the launcher's log files and handed to whoever needs a [`Build`]'s public
/// patch; an empty value gives no Build a public patch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublicPatches {
    by_change: HashMap<String, String>,
}

impl PublicPatches {
    /// Reads the public patches from the RSI Launcher's own log folder: `P4KX_LAUNCHER_LOGS`
    /// when set, otherwise `logs` in the launcher's application-data folder.
    ///
    /// Only `log.log` and `log.old.log` are read; a missing or unreadable file is skipped.
    #[must_use]
    pub fn standard() -> Self {
        launcher_log::standard_log_folder()
            .map_or_else(Self::default, |folder| Self::read_from(&folder))
    }

    /// Reads the public patches from the log files in `log_folder`.
    ///
    /// A line of the form `(SC <CHANNEL> <version>-<tag>.<change>)` names the version of that
    /// change; the newest line wins. At most the last 16 MiB of each file are read.
    #[must_use]
    pub fn read_from(log_folder: &Path) -> Self {
        launcher_log::scan(log_folder, Wanted::PATCHES).patches
    }

    /// Returns the public patch version, e.g. `4.7.2`, of the Build made from the change
    /// `change_number`, or `None` when no log line names that change.
    #[must_use]
    pub fn get(&self, change_number: &str) -> Option<&str> {
        self.by_change.get(change_number).map(String::as_str)
    }

    /// Returns `build` with the public patch of its change number, or without one when the log
    /// has no line for it.
    #[must_use]
    pub fn attach(&self, build: &Build) -> Build {
        build
            .clone()
            .with_public_patch(self.get(build.change_number()).map(str::to_owned))
    }

    /// Returns the public patch of `build`, without cloning the Build.
    #[must_use]
    pub fn of(&self, build: &Build) -> Option<&str> {
        self.get(build.change_number())
    }

    pub(crate) fn note_line(&mut self, line: &str) {
        let mut rest = line;
        while let Some((_, after)) = rest.split_once(MARKER) {
            let Some((label, tail)) = after.split_once(')') else {
                break;
            };
            if let Some((version, change)) = patch_of(label) {
                self.record(change, version);
            }
            rest = tail;
        }
    }

    pub(crate) fn record(&mut self, change: &str, version: &str) {
        if let Some(known) = self.by_change.get_mut(change) {
            version.clone_into(known);
        } else if self.by_change.len() < MAX_PATCHES {
            self.by_change.insert(change.to_owned(), version.to_owned());
        }
    }

    pub(crate) fn add_older(&mut self, older: Self) {
        for (change, version) in older.by_change {
            if self.by_change.len() >= MAX_PATCHES {
                break;
            }
            self.by_change.entry(change).or_insert(version);
        }
    }
}

/// Returns the version and the change of a label `<CHANNEL> <version>-<tag>.<change>`, e.g.
/// `4.7.2` and `11674325` for `LIVE 4.7.2-live.11674325`.
fn patch_of(label: &str) -> Option<(&str, &str)> {
    let (_channel, build) = label.split_once(' ')?;
    let (version, tail) = build.split_once('-')?;
    let (_tag, change) = tail.rsplit_once('.')?;
    (is_version(version) && is_change(change)).then_some((version, change))
}

fn is_version(version: &str) -> bool {
    version.len() <= MAX_VERSION_LEN
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn is_change(change: &str) -> bool {
    (1..=MAX_CHANGE_LEN).contains(&change.len()) && change.bytes().all(|byte| byte.is_ascii_digit())
}
