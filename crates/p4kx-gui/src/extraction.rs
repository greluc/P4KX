mod job;
mod summary;

use std::path::{Path, PathBuf};

use p4kx_extraction::ConflictPolicy;

pub use job::{Extraction, ExtractionStatus};
pub(crate) use job::{Request, start, target_naming};
pub use summary::{ExtractionSummary, FailedEntry};

/// Which Entries an Extraction writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionScope {
    /// The checked Entries of the whole Archive.
    Selected,
    /// The Entries shown in the table.
    Shown,
    /// Every Entry of the Archive.
    All,
}

/// Where an Extraction writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TargetKind {
    /// A new folder `SC-EXPORT-<Channel>-<Build>-<timestamp>` created inside the chosen folder.
    #[default]
    NewFolder,
    /// The chosen folder itself, with a Conflict policy for files already there.
    ExistingFolder,
}

/// What a whole-Archive Extraction asks to confirm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confirmation {
    /// The number of Entries, every Entry of the Archive.
    pub entries: usize,
    /// Their uncompressed size in bytes.
    pub bytes: u64,
}

/// The choices of an Extraction before it starts: scope, target and Conflict policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionSetup {
    scope: ExtractionScope,
    kind: TargetKind,
    folder: Option<PathBuf>,
    policy: ConflictPolicy,
    confirmation: Option<Confirmation>,
}

impl ExtractionSetup {
    pub(crate) const fn new(scope: ExtractionScope, folder: Option<PathBuf>) -> Self {
        Self {
            scope,
            kind: TargetKind::NewFolder,
            folder,
            policy: ConflictPolicy::Skip,
            confirmation: None,
        }
    }

    /// Returns which Entries are extracted.
    #[must_use]
    pub const fn scope(&self) -> ExtractionScope {
        self.scope
    }

    /// Chooses which Entries are extracted, withdrawing a pending confirmation.
    pub const fn set_scope(&mut self, scope: ExtractionScope) {
        self.scope = scope;
        self.confirmation = None;
    }

    /// Returns where the Extraction writes.
    #[must_use]
    pub const fn kind(&self) -> TargetKind {
        self.kind
    }

    /// Chooses where the Extraction writes.
    pub const fn set_kind(&mut self, kind: TargetKind) {
        self.kind = kind;
    }

    /// Returns the chosen folder, or `None` while none is chosen.
    #[must_use]
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Chooses the folder the new folder is created in, or the existing folder written into.
    pub fn set_folder(&mut self, folder: PathBuf) {
        self.folder = Some(folder);
    }

    /// Returns the Conflict policy for files already in an existing folder.
    #[must_use]
    pub const fn policy(&self) -> ConflictPolicy {
        self.policy
    }

    /// Chooses the Conflict policy for files already in an existing folder.
    pub const fn set_policy(&mut self, policy: ConflictPolicy) {
        self.policy = policy;
    }

    /// Returns whether a whole-Archive Extraction waits for confirmation.
    #[must_use]
    pub const fn is_confirming(&self) -> bool {
        self.confirmation.is_some()
    }

    /// Returns what a whole-Archive Extraction waits to confirm, or `None` when it does not.
    #[must_use]
    pub const fn confirmation(&self) -> Option<Confirmation> {
        self.confirmation
    }

    /// Withdraws a pending confirmation, back to the choices.
    pub const fn withdraw_confirmation(&mut self) {
        self.confirmation = None;
    }

    pub(crate) const fn ask_confirmation(&mut self, confirmation: Confirmation) {
        self.confirmation = Some(confirmation);
    }

    pub(crate) fn request(&self, ids: Vec<u32>) -> Option<Request> {
        Some(Request {
            ids,
            kind: self.kind,
            folder: self.folder.clone()?,
            policy: match self.kind {
                TargetKind::NewFolder => ConflictPolicy::Skip,
                TargetKind::ExistingFolder => self.policy,
            },
        })
    }
}
