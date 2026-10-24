// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use p4kx_assembly::character::{CHF_SIZE, Character, Chf};
use p4kx_assembly::{BodyVariant, Head};

use crate::EntryIndex;
use crate::model::describe;

/// Where the CHF file of a head comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadSource {
    /// An Entry of the Archive, by its path.
    Archive(String),
    /// A file the user picked.
    File(PathBuf),
}

/// The head the Character builder wears: the CHF it was read from, the body it belongs on and
/// the head rig that shapes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadInfo {
    pub(super) name: String,
    pub(super) source: HeadSource,
    pub(super) variant: BodyVariant,
    pub(super) rig: &'static str,
}

impl HeadInfo {
    /// Returns the file name of the CHF.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns where the CHF comes from.
    #[must_use]
    pub const fn source(&self) -> &HeadSource {
        &self.source
    }

    /// Returns the body variant the head belongs on.
    #[must_use]
    pub const fn variant(&self) -> BodyVariant {
        self.variant
    }

    /// Returns the name of the head rig, e.g. `protos_human_male_face_t1_pu`.
    #[must_use]
    pub const fn rig(&self) -> &'static str {
        self.rig
    }
}

/// A head chosen and read: what is shown and the Character to make the head of.
#[derive(Debug, Clone)]
pub(super) struct HeadPick {
    pub(super) info: HeadInfo,
    pub(super) character: Arc<Character>,
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn read_capped(reader: impl Read) -> Result<Vec<u8>, String> {
    let limit = u64::try_from(CHF_SIZE).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| describe(&error))?;
    if u64::try_from(bytes.len()).is_ok_and(|len| len > limit) {
        return Err(format!("the file is larger than {limit} bytes"));
    }
    Ok(bytes)
}

fn open(source: &HeadSource, index: &EntryIndex) -> Result<(String, Vec<u8>), String> {
    match source {
        HeadSource::Archive(path) => {
            let entry = index
                .id_of_path(path)
                .and_then(|id| index.entry(id))
                .ok_or_else(|| format!("the Archive has no Entry {path}"))?;
            let reader = index
                .archive()
                .open_entry(entry)
                .map_err(|error| describe(&error))?;
            Ok((file_name(path).to_owned(), read_capped(reader)?))
        }
        HeadSource::File(path) => {
            let file = File::open(path).map_err(|error| describe(&error))?;
            let name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            Ok((name, read_capped(file)?))
        }
    }
}

/// Reads the CHF of `source` and makes the head of its Character, to learn its body and rig.
pub(super) fn read(source: &HeadSource, index: &EntryIndex) -> Result<HeadPick, String> {
    let (name, bytes) = open(source, index)?;
    let chf = Chf::read(&bytes).map_err(|error| describe(&error))?;
    let head = Head::from_character(&chf.character).map_err(|error| describe(&error))?;
    Ok(HeadPick {
        info: HeadInfo {
            name,
            source: source.clone(),
            variant: head.variant(),
            rig: head.rig(),
        },
        character: Arc::new(chf.character),
    })
}

/// Returns the path of a file the user picked, for the facts of a head.
pub(super) fn picked(path: &Path) -> HeadSource {
    HeadSource::File(path.to_path_buf())
}
