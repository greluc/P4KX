// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_foundation::ArchiveKey;

/// How [`Archive::open_with`](crate::Archive::open_with) opens an Archive. The default is what
/// [`Archive::open`](crate::Archive::open) does.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct OpenOptions {
    pub(crate) recover: bool,
    pub(crate) key: Option<ArchiveKey>,
}

impl OpenOptions {
    /// Returns the default options: no recovery, no key.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether an Archive whose end records are missing or whose central directory is
    /// damaged is rebuilt by scanning the last [`MAX_RECOVERY_SCAN`](crate::MAX_RECOVERY_SCAN)
    /// bytes of the file for directory records.
    ///
    /// Only records whose local header agrees are kept, at most
    /// [`MAX_RECOVERED_ENTRIES`](crate::MAX_RECOVERED_ENTRIES) of them; Entries whose records are
    /// lost are missing, and [`EntryIssue::Recovered`](crate::EntryIssue::Recovered) says so. An
    /// intact Archive opens as usual; the v2 layout is not recovered.
    #[must_use]
    pub const fn recover(mut self, recover: bool) -> Self {
        self.recover = recover;
        self
    }

    /// Sets the key of the Archive's encrypted Entries, see
    /// [`Archive::set_key`](crate::Archive::set_key).
    #[must_use]
    pub const fn key(mut self, key: Option<ArchiveKey>) -> Self {
        self.key = key;
        self
    }
}
