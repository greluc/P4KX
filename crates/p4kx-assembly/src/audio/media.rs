use std::collections::HashMap;

use p4kx_archive::Entry;

use super::language::{LooseName, bank_language, loose_name};
use super::read::folded_order;
use super::{AudioFileError, MAX_LANGUAGES, MAX_LOOSE_WEMS, WemLocation};

/// A loose WEM Entry of the audio index.
#[derive(Debug)]
struct LooseWem {
    path: Box<str>,
    language: u16,
    len: u64,
}

const NO_LANGUAGE: u16 = u16::MAX;

/// The loose WEM Entries below `Data\Sounds\wwise\`, found by the ID in their file name, and the
/// language folders of the soundbank and WEM Entries there.
#[derive(Debug, Default)]
pub(crate) struct LooseMedia {
    languages: Vec<Box<str>>,
    wems: HashMap<u32, Vec<LooseWem>>,
}

impl LooseMedia {
    /// Collects the loose WEMs among `wem_entries`, which are sorted by path, and the languages of
    /// them and of the soundbank folders `bank_paths`.
    ///
    /// Returns the error that left Entries out, if any: past [`MAX_LOOSE_WEMS`] WEMs or
    /// [`MAX_LANGUAGES`] languages further ones are dropped.
    pub(crate) fn collect<'a>(
        wem_entries: &[&'a Entry],
        bank_paths: impl Iterator<Item = &'a str>,
    ) -> (Self, Option<(Box<str>, AudioFileError)>) {
        let mut media = Self::default();
        let mut failure = None;
        for path in bank_paths {
            if let Some(language) = bank_language(path) {
                media.language(language);
            }
        }
        let mut count = 0_usize;
        for entry in wem_entries {
            let Some(LooseName { id, language }) = loose_name(entry.path()) else {
                continue;
            };
            let language = language.map_or(Some(NO_LANGUAGE), |name| media.language(name));
            let Some(language) = language.filter(|_| count < MAX_LOOSE_WEMS) else {
                failure.get_or_insert_with(|| (entry.path().into(), AudioFileError::TooManyMedia));
                continue;
            };
            count += 1;
            media.wems.entry(id).or_default().push(LooseWem {
                path: entry.path().into(),
                language,
                len: entry.uncompressed_size(),
            });
        }
        media.sort_languages();
        (media, failure)
    }

    fn sort_languages(&mut self) {
        let mut order: Vec<usize> = (0..self.languages.len()).collect();
        order.sort_by(|&a, &b| {
            let name = |at: usize| self.languages.get(at).map_or("", |name| &**name);
            folded_order(name(a), name(b))
        });
        let mut renumbered = vec![NO_LANGUAGE; order.len()];
        for (new, &old) in order.iter().enumerate() {
            if let Some(slot) = renumbered.get_mut(old) {
                *slot = u16::try_from(new).unwrap_or(NO_LANGUAGE);
            }
        }
        for wem in self.wems.values_mut().flatten() {
            if let Some(&new) = renumbered.get(usize::from(wem.language)) {
                wem.language = new;
            }
        }
        let mut sorted: Vec<Option<Box<str>>> = std::mem::take(&mut self.languages)
            .into_iter()
            .map(Some)
            .collect();
        self.languages = order
            .iter()
            .filter_map(|&old| sorted.get_mut(old)?.take())
            .collect();
    }

    fn language(&mut self, name: &str) -> Option<u16> {
        if let Some(position) = self
            .languages
            .iter()
            .position(|known| known.eq_ignore_ascii_case(name))
        {
            return u16::try_from(position).ok();
        }
        if self.languages.len() >= MAX_LANGUAGES {
            return None;
        }
        self.languages.push(name.into());
        u16::try_from(self.languages.len() - 1).ok()
    }

    pub(crate) fn languages(&self) -> &[Box<str>] {
        &self.languages
    }

    pub(crate) fn language_of(&self, name: &str) -> Option<&str> {
        self.languages
            .iter()
            .find(|known| known.eq_ignore_ascii_case(name))
            .map(|known| &**known)
    }

    /// Returns the loose WEMs with the ID `id`, in path order, with their language folder.
    pub(crate) fn find(&self, id: u32) -> impl Iterator<Item = (Option<&str>, WemLocation<'_>)> {
        self.wems.get(&id).into_iter().flatten().map(|wem| {
            let language = self
                .languages
                .get(usize::from(wem.language))
                .map(|name| &**name);
            (
                language,
                WemLocation::Entry {
                    path: &wem.path,
                    language,
                    len: wem.len,
                },
            )
        })
    }
}
