// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::rc::Rc;

use p4kx_archive::Archive;
use p4kx_audio::Soundbank;

use super::read::entry_bytes;
use super::{
    AudioFileError, AudioIndex, BreakReason, IndexedSoundbank, LanguageChoice, MAX_SOUNDBANK_LEN,
};

/// A soundbank read from the Archive and parsed, shared by the walks that follow events in it.
#[derive(Debug, Clone)]
pub(crate) struct Loaded<'i> {
    pub(crate) position: u32,
    pub(crate) soundbank: &'i IndexedSoundbank,
    pub(crate) bank: Rc<Soundbank<Vec<u8>>>,
}

impl<'i> Loaded<'i> {
    /// Reads and parses the soundbank at `position` of the index.
    pub(crate) fn read(
        archive: &Archive,
        index: &'i AudioIndex,
        position: u32,
    ) -> Result<Self, AudioFileError> {
        let soundbank = soundbank_at(index, position).ok_or_else(gone)?;
        let entry = archive.entries().get(soundbank.entry).ok_or_else(gone)?;
        let bank = Soundbank::parse(entry_bytes(archive, entry, MAX_SOUNDBANK_LEN)?)?;
        Ok(Self {
            position,
            soundbank,
            bank: Rc::new(bank),
        })
    }
}

/// How a reference names the soundbank that holds its target.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Via {
    Local,
    Bank(u32),
    PlayEvent,
    At(u32),
}

#[derive(Debug)]
enum Extra<'i> {
    Unused,
    Loaded(Loaded<'i>),
    Failed(AudioFileError),
}

/// The soundbanks one event walk may be in: the one it starts in and at most one other, read on
/// the first reference that leaves the first.
#[derive(Debug)]
pub(crate) struct Banks<'w, 'i> {
    archive: &'w Archive,
    index: &'i AudioIndex,
    choice: &'w LanguageChoice,
    home: Loaded<'i>,
    extra: Extra<'i>,
}

impl<'w, 'i> Banks<'w, 'i> {
    pub(crate) const fn new(
        archive: &'w Archive,
        index: &'i AudioIndex,
        choice: &'w LanguageChoice,
        home: Loaded<'i>,
    ) -> Self {
        Self {
            archive,
            index,
            choice,
            home,
            extra: Extra::Unused,
        }
    }

    /// Returns the soundbank a reference to `id` leaving the soundbank at `from` goes to: the
    /// start soundbank or the one other soundbank, read now when it is the first to be needed.
    pub(crate) fn enter(
        &mut self,
        from: u32,
        id: u32,
        via: Via,
    ) -> Result<Loaded<'i>, BreakReason> {
        let candidates = self.candidates(id, via)?;
        if candidates.contains(&from) {
            return Err(BreakReason::Missing { id });
        }
        let Some(&wanted) = candidates.first() else {
            return Err(match via {
                Via::Bank(bank_id) => BreakReason::OtherSoundbank { id, bank_id },
                Via::Local | Via::PlayEvent | Via::At(_) => BreakReason::Missing { id },
            });
        };
        if let Some(loaded) = self.loaded(&candidates) {
            return Ok(loaded);
        }
        let bank_id = soundbank_at(self.index, wanted).map_or(0, IndexedSoundbank::id);
        match &self.extra {
            Extra::Failed(error) => Err(BreakReason::Unreadable(error.clone())),
            Extra::Loaded(_) => Err(BreakReason::OtherSoundbank { id, bank_id }),
            Extra::Unused if from != self.home.position => {
                Err(BreakReason::OtherSoundbank { id, bank_id })
            }
            Extra::Unused => match Loaded::read(self.archive, self.index, wanted) {
                Ok(loaded) => {
                    self.extra = Extra::Loaded(loaded.clone());
                    Ok(loaded)
                }
                Err(error) => {
                    self.extra = Extra::Failed(error.clone());
                    Err(BreakReason::Unreadable(error))
                }
            },
        }
    }

    fn loaded(&self, candidates: &[u32]) -> Option<Loaded<'i>> {
        let extra = match &self.extra {
            Extra::Loaded(loaded) => Some(loaded),
            _ => None,
        };
        [Some(&self.home), extra]
            .into_iter()
            .flatten()
            .find(|loaded| candidates.contains(&loaded.position))
            .cloned()
    }

    fn candidates(&self, id: u32, via: Via) -> Result<Vec<u32>, BreakReason> {
        let positions = match via {
            Via::Local | Via::Bank(0) => return Err(BreakReason::Missing { id }),
            Via::Bank(bank_id) => self.index.bank_positions(bank_id),
            Via::PlayEvent => self.index.event_holders(id),
            Via::At(position) => &[position],
        };
        let selected = select_holders(self.index, positions, self.choice);
        if selected.is_empty() && !positions.is_empty() {
            return Err(self.choice.no_localisation());
        }
        Ok(selected)
    }
}

/// Returns the soundbanks of `holders` an event is followed in: every soundbank that is not
/// localised, and of the localised ones those of the first language of `choice` that has any.
pub(crate) fn select_holders(
    index: &AudioIndex,
    holders: &[u32],
    choice: &LanguageChoice,
) -> Vec<u32> {
    if choice.is_any() {
        return holders.to_vec();
    }
    let language_of =
        |position: u32| soundbank_at(index, position).and_then(IndexedSoundbank::language);
    let mut selected: Vec<u32> = holders
        .iter()
        .copied()
        .filter(|&position| language_of(position).is_none())
        .collect();
    let localised = |wanted: &str| {
        holders
            .iter()
            .copied()
            .filter(|&position| {
                language_of(position).is_some_and(|language| language.eq_ignore_ascii_case(wanted))
            })
            .collect::<Vec<u32>>()
    };
    if let Some(chosen) = choice
        .candidates()
        .map(localised)
        .find(|banks| !banks.is_empty())
    {
        selected.extend(chosen);
        selected.sort_unstable();
    }
    selected
}

pub(crate) fn soundbank_at(index: &AudioIndex, position: u32) -> Option<&IndexedSoundbank> {
    index.soundbanks().get(usize::try_from(position).ok()?)
}

pub(crate) fn gone() -> AudioFileError {
    AudioFileError::Unreadable {
        reason: "the Archive no longer holds the soundbank Entry".to_owned(),
    }
}
