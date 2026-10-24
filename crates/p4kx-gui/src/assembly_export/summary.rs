// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use p4kx_assembly::{Assembly, Issue, LoadoutChanges};
use p4kx_extraction::GlbSummary;

use crate::model_info::printable;

/// A part of the Assembly that could not be resolved or done in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartIssue {
    /// The path of the part's port from the Entity, port names joined by `/`; empty for the
    /// Entity itself.
    pub path: String,
    /// The class name of the part's Entity.
    pub name: String,
    /// What went wrong.
    pub reason: String,
    /// Whether the part, or its geometry, is left out of the file.
    pub skipped: bool,
}

impl PartIssue {
    pub(crate) fn of(issue: &Issue) -> Self {
        Self {
            path: printable(issue.path()),
            name: printable(issue.name()),
            reason: printable(&issue.kind().to_string()),
            skipped: issue.kind().skips(),
        }
    }
}

/// What a finished Export of an Entity as GLB wrote and what it left out.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyExportSummary {
    file: PathBuf,
    name: String,
    lod: u32,
    parts: usize,
    modelled: usize,
    written: GlbSummary,
    issues: Vec<PartIssue>,
    excluded: usize,
    replaced: usize,
}

impl AssemblyExportSummary {
    pub(crate) fn new(
        (file, name): (PathBuf, String),
        assembly: &Assembly,
        written: GlbSummary,
        changes: &LoadoutChanges,
    ) -> Self {
        Self {
            file,
            name,
            lod: assembly.lod(),
            parts: assembly.parts().len(),
            modelled: assembly.modelled_parts(),
            written,
            issues: assembly.issues().iter().map(PartIssue::of).collect(),
            excluded: changes.exclusions().len(),
            replaced: changes.replacements().len(),
        }
    }

    /// Returns the GLB file that was written.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the class name of the Entity.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the level of detail that was exported.
    #[must_use]
    pub const fn lod(&self) -> u32 {
        self.lod
    }

    /// Returns how many parts the Assembly holds.
    #[must_use]
    pub const fn parts(&self) -> usize {
        self.parts
    }

    /// Returns how many parts have a model.
    #[must_use]
    pub const fn modelled_parts(&self) -> usize {
        self.modelled
    }

    /// Returns what the writer counted.
    #[must_use]
    pub const fn written(&self) -> &GlbSummary {
        &self.written
    }

    /// Returns the parts that could not be resolved or done in full, in the order the Assembly lists them.
    #[must_use]
    pub fn issues(&self) -> &[PartIssue] {
        &self.issues
    }

    /// Returns how many parts the report lists as left out of the file.
    #[must_use]
    pub fn skipped(&self) -> usize {
        self.issues.iter().filter(|issue| issue.skipped).count()
    }

    /// Returns how many ports the changes left out.
    #[must_use]
    pub const fn excluded(&self) -> usize {
        self.excluded
    }

    /// Returns how many ports hold another Entity because of the changes.
    #[must_use]
    pub const fn replaced(&self) -> usize {
        self.replaced
    }
}
