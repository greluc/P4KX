use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde::{Serialize, Serializer};

use crate::entry_change::{count, count_by_directory};
use crate::{
    Archive, Build, DirectoryCounts, EntryChange, EntryCounts, Filter, Levels, Report,
    report::to_report,
};

/// One of the two Archives of a Comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side<'a> {
    archive: &'a Path,
    channel: Option<String>,
    build: Option<&'a Build>,
}

impl<'a> Side<'a> {
    pub(crate) fn of(archive: &'a Archive) -> Self {
        Self {
            archive: archive.path(),
            channel: p4kx_channels::channel_name(archive.path()),
            build: archive.build(),
        }
    }

    /// Returns the path the Archive was opened from.
    #[must_use]
    pub const fn archive(&self) -> &'a Path {
        self.archive
    }

    /// Returns the Channel: the name of the folder holding the Archive.
    #[must_use]
    pub fn channel(&self) -> Option<&str> {
        self.channel.as_deref()
    }

    /// Returns the Build, or `None` when it is unknown.
    #[must_use]
    pub const fn build(&self) -> Option<&'a Build> {
        self.build
    }
}

/// The result of comparing an old with a new Archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison<'a> {
    pub(crate) old: Side<'a>,
    pub(crate) new: Side<'a>,
    pub(crate) levels: Levels,
    pub(crate) filter: Option<String>,
    pub(crate) entries: Vec<EntryChange<'a>>,
}

impl<'a> Comparison<'a> {
    /// Returns the old side.
    #[must_use]
    pub const fn old_side(&self) -> &Side<'a> {
        &self.old
    }

    /// Returns the new side.
    #[must_use]
    pub const fn new_side(&self) -> &Side<'a> {
        &self.new
    }

    /// Returns the levels that were compared.
    #[must_use]
    pub const fn levels(&self) -> Levels {
        self.levels
    }

    /// Returns the text of the Filter that selected the Entries, `None` when every Entry was
    /// compared.
    #[must_use]
    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    /// Returns the added, removed and changed Entries, ordered by normalised path.
    #[must_use]
    pub fn entry_changes(&self) -> &[EntryChange<'a>] {
        &self.entries
    }

    /// Returns the number of changed Entries per change kind.
    #[must_use]
    pub fn entry_counts(&self) -> EntryCounts {
        count(&self.entries)
    }

    /// Returns the number of changed Entries per change kind below each top-level directory that
    /// has a change, ordered by normalised directory.
    #[must_use]
    pub fn directory_counts(&self) -> Vec<DirectoryCounts<'a>> {
        count_by_directory(&self.entries)
    }

    /// Returns whether any compared level found a difference.
    #[must_use]
    pub const fn has_differences(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Returns the JSON report of the Comparison.
    #[must_use]
    pub fn report(&self) -> Report<'_> {
        to_report(self)
    }

    /// Returns the Comparison of only the Entries whose path matches `filter`, the same as
    /// comparing with that Filter in the first place.
    ///
    /// A report names one Filter, so a Comparison that was already compared or narrowed with a
    /// Filter yields `None`.
    #[must_use]
    pub fn filtered(&self, filter: &Filter) -> Option<Self> {
        if self.filter.is_some() {
            return None;
        }
        Some(Self {
            old: self.old.clone(),
            new: self.new.clone(),
            levels: self.levels,
            filter: Some(filter.as_str().to_owned()),
            entries: self
                .entries
                .iter()
                .filter(|change| filter.matches(change.path()))
                .copied()
                .collect(),
        })
    }

    /// Writes the JSON report to the file at `path` as pretty JSON with a final newline and
    /// flushes it to disk.
    ///
    /// # Errors
    ///
    /// Returns the error of creating, writing or syncing the file.
    pub fn save_report(&self, path: &Path) -> io::Result<()> {
        let mut out = BufWriter::new(File::create(path)?);
        serde_json::to_writer_pretty(&mut out, self)?;
        writeln!(out)?;
        out.into_inner()
            .map_err(io::IntoInnerError::into_error)?
            .sync_all()
    }
}

impl Serialize for Comparison<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.report().serialize(serializer)
    }
}
