use p4kx_archive::{Archive, Entry};
use p4kx_localization::{Language, TextSet};
use rayon::prelude::*;

use crate::localization::{LocalizationEntry, LocalizationLoadError};

/// The `global.ini` Entries of several languages of an Archive, read into memory.
///
/// A language the Archive holds no `global.ini` for is left out, not an error; the Entries are
/// read in parallel.
#[derive(Debug)]
pub struct LocalizationSet {
    entries: Vec<LocalizationEntry>,
}

impl LocalizationSet {
    /// Reads the `global.ini` of every language in `wanted` that `archive` holds.
    ///
    /// # Errors
    ///
    /// Returns the first [`LocalizationLoadError`] of an Entry that cannot be read.
    pub fn read(archive: &Archive, wanted: &[Language]) -> Result<Self, LocalizationLoadError> {
        let located: Vec<(Language, &Entry)> = archive
            .entries()
            .par_iter()
            .filter_map(|entry| {
                let language =
                    Language::from_global_ini_path(entry.path()).filter(|l| wanted.contains(l))?;
                Some((language, entry))
            })
            .collect();
        let mut first_of_each: Vec<(Language, &Entry)> = Vec::with_capacity(located.len());
        for (language, entry) in located {
            if first_of_each.iter().all(|(known, _)| *known != language) {
                first_of_each.push((language, entry));
            }
        }
        first_of_each.sort_by_key(|(language, _)| *language);
        let entries = first_of_each
            .into_par_iter()
            .map(|(language, entry)| LocalizationEntry::read_entry(archive, language, entry))
            .collect::<Result<_, _>>()?;
        Ok(Self { entries })
    }

    /// Reads the `global.ini` of every language the Archive holds.
    ///
    /// # Errors
    ///
    /// Returns the first [`LocalizationLoadError`] of an Entry that cannot be read.
    pub fn read_all(archive: &Archive) -> Result<Self, LocalizationLoadError> {
        Self::read(archive, &Language::ALL)
    }

    /// Returns the languages that were read, in the order of [`Language::ALL`].
    pub fn languages(&self) -> impl Iterator<Item = Language> {
        self.entries.iter().map(LocalizationEntry::language)
    }

    /// Returns the Entry that was read for `language`.
    #[must_use]
    pub fn entry(&self, language: Language) -> Option<&LocalizationEntry> {
        self.entries
            .iter()
            .find(|entry| entry.language() == language)
    }

    /// Returns the Entries that were read, in the order of [`Language::ALL`].
    #[must_use]
    pub fn entries(&self) -> &[LocalizationEntry] {
        &self.entries
    }

    /// Indexes every Entry in parallel and returns the set to resolve keys in.
    ///
    /// # Errors
    ///
    /// Returns [`LocalizationLoadError::Index`] for the first Entry that exceeds a limit.
    pub fn index(&self) -> Result<TextSet<'_>, LocalizationLoadError> {
        let files = self
            .entries
            .par_iter()
            .map(|entry| {
                let ini = entry
                    .parse()
                    .map_err(|source| LocalizationLoadError::Index {
                        path: entry.path().to_owned(),
                        source,
                    })?;
                Ok((entry.language(), ini))
            })
            .collect::<Result<Vec<_>, LocalizationLoadError>>()?;
        Ok(TextSet::new(files))
    }
}
