use rayon::prelude::*;

use crate::decode::{Decoded, Encoding, decode};
use crate::path_key::eq_normalised;
use crate::{Archive, ChangeKind, Entry, LineDiffError, TextDiff};

/// Why an Entry pair has no line diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotText {
    /// A side holds a NUL byte, so it is binary.
    Binary,
    /// A side, or the XML converted from it, is larger than
    /// [`MAX_DIFF_LEN`](crate::MAX_DIFF_LEN).
    TooLarge,
}

/// What the line diff of an Entry pair holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffBody {
    /// Both sides are text; a missing side is empty.
    Text(TextDiff),
    /// A side is not diffable line by line.
    NotText(NotText),
}

/// The line diff of one Entry between an old and a new Archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDiff<'a> {
    old: Option<&'a Entry>,
    new: Option<&'a Entry>,
    old_encoding: Option<Encoding>,
    new_encoding: Option<Encoding>,
    body: DiffBody,
}

impl<'a> LineDiff<'a> {
    /// Returns the Entry path as the new side stores it, or as the old side does for a removed
    /// Entry.
    #[must_use]
    pub fn path(&self) -> &'a str {
        self.new.or(self.old).map_or("", Entry::path)
    }

    /// Returns how the Entry differs by its central-directory size and CRC-32C, `None` when it
    /// is unchanged.
    #[must_use]
    pub fn kind(&self) -> Option<ChangeKind> {
        match (self.old, self.new) {
            (Some(old), Some(new)) => (old.uncompressed_size() != new.uncompressed_size()
                || old.crc32c() != new.crc32c())
            .then_some(ChangeKind::Changed),
            (Some(_), None) => Some(ChangeKind::Removed),
            (None, _) => Some(ChangeKind::Added),
        }
    }

    /// Returns the Entry of the old side, `None` when it was added.
    #[must_use]
    pub const fn old_entry(&self) -> Option<&'a Entry> {
        self.old
    }

    /// Returns the Entry of the new side, `None` when it was removed.
    #[must_use]
    pub const fn new_entry(&self) -> Option<&'a Entry> {
        self.new
    }

    /// Returns how the old side was read, `None` when it is missing or not text.
    #[must_use]
    pub const fn old_encoding(&self) -> Option<Encoding> {
        self.old_encoding
    }

    /// Returns how the new side was read, `None` when it is missing or not text.
    #[must_use]
    pub const fn new_encoding(&self) -> Option<Encoding> {
        self.new_encoding
    }

    /// Returns the hunks, or why there are none.
    #[must_use]
    pub const fn body(&self) -> &DiffBody {
        &self.body
    }

    /// Returns whether the Entry differs: added, removed, with differing lines, or not text and
    /// changed.
    #[must_use]
    pub fn has_differences(&self) -> bool {
        match (&self.body, self.kind()) {
            (_, Some(ChangeKind::Added | ChangeKind::Removed)) => true,
            (DiffBody::Text(text), _) => text.has_differences(),
            (DiffBody::NotText(_), kind) => kind.is_some(),
        }
    }
}

/// Returns the line diff of the Entry at `path` between the `old` and the `new` Archive, with
/// `context` unchanged lines around each change.
///
/// The path matches with either separator and ignoring ASCII case. `CryXml` is converted to XML
/// first, UTF-16 text with a byte order mark is decoded, and other text is read as UTF-8; a side
/// with a NUL byte is binary. A missing side is diffed as empty text. The patience diff falls back
/// to Myers where no line is unique; a region needing more than 1024 edits there, or any region
/// once a fixed work budget is spent, is shown as replaced as a whole, so the work stays bounded.
///
/// # Errors
///
/// Returns [`LineDiffError::UnknownEntry`] when neither Archive has the Entry, and another
/// [`LineDiffError`] when a side cannot be read or its `CryXml` is corrupt.
pub fn line_diff<'a>(
    old: &'a Archive,
    new: &'a Archive,
    path: &str,
    context: usize,
) -> Result<LineDiff<'a>, LineDiffError> {
    let (old_entry, new_entry) = rayon::join(|| find(old, path), || find(new, path));
    if old_entry.is_none() && new_entry.is_none() {
        return Err(LineDiffError::UnknownEntry {
            path: path.to_owned(),
        });
    }
    let (old_side, new_side) = rayon::join(
        || old_entry.map(|entry| decode(old, entry)).transpose(),
        || new_entry.map(|entry| decode(new, entry)).transpose(),
    );
    let (old_side, new_side) = (old_side?, new_side?);
    let encoding = |side: &Option<Decoded>| match side {
        Some(Decoded::Text(_, encoding)) => Some(*encoding),
        _ => None,
    };
    let (old_encoding, new_encoding) = (encoding(&old_side), encoding(&new_side));
    let body = match (old_side, new_side) {
        (Some(Decoded::TooLarge), _) | (_, Some(Decoded::TooLarge)) => {
            DiffBody::NotText(NotText::TooLarge)
        }
        (Some(Decoded::Binary), _) | (_, Some(Decoded::Binary)) => {
            DiffBody::NotText(NotText::Binary)
        }
        (old_side, new_side) => {
            DiffBody::Text(TextDiff::new(text_of(old_side), text_of(new_side), context))
        }
    };
    Ok(LineDiff {
        old: old_entry,
        new: new_entry,
        old_encoding,
        new_encoding,
        body,
    })
}

fn text_of(side: Option<Decoded>) -> String {
    match side {
        Some(Decoded::Text(text, _)) => text,
        _ => String::new(),
    }
}

fn find<'a>(archive: &'a Archive, path: &str) -> Option<&'a Entry> {
    archive
        .entries()
        .par_iter()
        .find_first(|entry| eq_normalised(entry.path(), path))
}
