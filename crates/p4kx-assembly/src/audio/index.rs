// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::collections::hash_map::Entry as Slot;

use p4kx_archive::{Archive, Entry};
use p4kx_audio::MusicStinger;
use rayon::prelude::*;

use super::banks::BankContents;
use super::language::SOUND_FOLDER;
use super::media::{Limits, LooseMedia, WemSource};
use super::read::{entry_bytes, folded_order, has_extension, starts_with_folder};
use super::{
    ATL_FOLDER, AtlTrigger, AudioFileError, AudioIssue, DEFAULT_LANGUAGE, IndexedSoundbank,
    MAX_ATL_LEN, MAX_AUDIO_TRIGGERS, MAX_SOUNDBANK_LEN, MAX_STINGERS, WemLocation, atl, banks,
};

type BankRead = Result<BankContents, AudioFileError>;

/// The audio data of one Archive: every ATL trigger with its Wwise events, and for every Wwise
/// event ID the soundbanks whose `HIRC` section holds the event.
///
/// [`AudioIndex::build`] reads the ATL definitions below [`ATL_FOLDER`] and every `.bnk` Entry
/// once, in parallel; an Entry that cannot be read is left out and kept as an [`AudioIssue`].
/// Triggers are looked up by name ignoring ASCII case, events by ID; when several definitions
/// name the same trigger, the first by Entry path wins.
#[derive(Debug, Default)]
pub struct AudioIndex {
    atl_files: Vec<Box<str>>,
    triggers: Vec<AtlTrigger>,
    by_name: HashMap<Box<str>, usize>,
    soundbanks: Vec<IndexedSoundbank>,
    events: HashMap<u32, Box<[u32]>>,
    bank_ids: HashMap<u32, Box<[u32]>>,
    stingers: HashMap<u32, Box<[Stinger]>>,
    media: HashMap<u32, Box<[MediaSpot]>>,
    loose: LooseMedia,
    issues: Vec<AudioIssue>,
}

impl AudioIndex {
    /// Reads the ATL definitions and soundbanks of `archive` into an index.
    #[must_use]
    pub fn build(archive: &Archive) -> Self {
        let mut atl_entries: Vec<&Entry> = archive
            .entries()
            .iter()
            .filter(|entry| {
                starts_with_folder(entry.path(), ATL_FOLDER) && has_extension(entry.path(), "xml")
            })
            .collect();
        let mut bank_entries: Vec<(usize, &Entry)> = archive
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| has_extension(entry.path(), "bnk"))
            .collect();
        let mut wem_entries: Vec<WemSource<'_>> = archive
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                starts_with_folder(entry.path(), SOUND_FOLDER) && has_extension(entry.path(), "wem")
            })
            .map(|(position, entry)| WemSource {
                entry: u32::try_from(position).unwrap_or(u32::MAX),
                path: entry.path(),
                len: entry.uncompressed_size(),
            })
            .collect();
        atl_entries.sort_by(|a, b| folded_order(a.path(), b.path()));
        bank_entries.sort_by(|(_, a), (_, b)| folded_order(a.path(), b.path()));
        wem_entries.sort_by(|a, b| folded_order(a.path, b.path));
        let definitions: Vec<_> = atl_entries
            .par_iter()
            .enumerate()
            .map(|(position, entry)| {
                let file = u32::try_from(position).unwrap_or(u32::MAX);
                entry_bytes(archive, entry, MAX_ATL_LEN).and_then(|bytes| atl::parse(&bytes, file))
            })
            .collect();
        let soundbanks: Vec<BankRead> = bank_entries
            .par_iter()
            .map(|&(position, entry)| {
                entry_bytes(archive, entry, MAX_SOUNDBANK_LEN)
                    .and_then(|bytes| banks::read(entry.path(), position, &bytes))
            })
            .collect();
        let mut index = Self::default();
        index.add_definitions(&atl_entries, definitions);
        index.add_soundbanks(&bank_entries, soundbanks);
        let (loose, failures) = LooseMedia::collect(
            wem_entries.iter().copied(),
            bank_entries.iter().map(|(_, entry)| entry.path()),
            Limits::DEFAULT,
        );
        index.loose = loose;
        index.issues.extend(
            failures
                .into_iter()
                .map(|(path, error)| AudioIssue { path, error }),
        );
        index
    }

    fn add_definitions(
        &mut self,
        entries: &[&Entry],
        definitions: Vec<Result<Vec<AtlTrigger>, AudioFileError>>,
    ) {
        self.atl_files = entries.iter().map(|entry| entry.path().into()).collect();
        for (entry, definition) in entries.iter().zip(definitions) {
            let error = match definition {
                Ok(triggers) => self.add_triggers(triggers).err(),
                Err(error) => Some(error),
            };
            if let Some(error) = error {
                self.issues.push(AudioIssue {
                    path: entry.path().into(),
                    error,
                });
            }
        }
    }

    fn add_triggers(&mut self, triggers: Vec<AtlTrigger>) -> Result<(), AudioFileError> {
        for trigger in triggers {
            if let Slot::Vacant(slot) = self.by_name.entry(trigger.name.to_ascii_lowercase().into())
            {
                if self.triggers.len() >= MAX_AUDIO_TRIGGERS {
                    return Err(AudioFileError::TooManyTriggers);
                }
                slot.insert(self.triggers.len());
                self.triggers.push(trigger);
            }
        }
        Ok(())
    }

    fn add_soundbanks(&mut self, entries: &[(usize, &Entry)], soundbanks: Vec<BankRead>) {
        let mut events: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut media: HashMap<u32, Vec<MediaSpot>> = HashMap::new();
        let mut bank_ids: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut stingers = StingerList::default();
        for ((_, entry), soundbank) in entries.iter().zip(soundbanks) {
            match soundbank {
                Ok(contents) => {
                    let position = u32::try_from(self.soundbanks.len()).unwrap_or(u32::MAX);
                    bank_ids
                        .entry(contents.soundbank.id())
                        .or_default()
                        .push(position);
                    self.soundbanks.push(contents.soundbank);
                    stingers.add(position, &contents.stingers);
                    for id in contents.events {
                        let holders = events.entry(id).or_default();
                        if holders.last() != Some(&position) {
                            holders.push(position);
                        }
                    }
                    for wem in contents.media {
                        media.entry(wem.id).or_default().push(MediaSpot {
                            soundbank: position,
                            offset: wem.offset,
                            len: wem.len,
                        });
                    }
                }
                Err(error) => self.issues.push(AudioIssue {
                    path: entry.path().into(),
                    error,
                }),
            }
        }
        self.events = boxed(events);
        self.bank_ids = boxed(bank_ids);
        self.stingers = boxed(stingers.found);
        self.media = boxed(media);
    }

    /// Returns the trigger called `name`, ignoring ASCII case, or `None` when no ATL definition
    /// has it.
    #[must_use]
    pub fn trigger(&self, name: &str) -> Option<&AtlTrigger> {
        self.by_name
            .get(name.to_ascii_lowercase().as_str())
            .and_then(|&position| self.triggers.get(position))
    }

    /// Returns every trigger, in the order of their definitions' Entry paths.
    #[must_use]
    pub fn triggers(&self) -> &[AtlTrigger] {
        &self.triggers
    }

    /// Returns the path of the ATL definition Entry that defines `trigger`, or `None` for a
    /// trigger of another index.
    #[must_use]
    pub fn definition_path(&self, trigger: &AtlTrigger) -> Option<&str> {
        let file = usize::try_from(trigger.file).ok()?;
        self.atl_files.get(file).map(|path| &**path)
    }

    /// Returns every soundbank that was parsed, in path order.
    #[must_use]
    pub fn soundbanks(&self) -> &[IndexedSoundbank] {
        &self.soundbanks
    }

    /// Returns the soundbanks whose `HIRC` section holds the Event object `id`, in path order.
    pub fn event_soundbanks(&self, id: u32) -> impl Iterator<Item = &IndexedSoundbank> {
        self.soundbanks_at(self.event_holders(id))
    }

    pub(crate) fn soundbanks_at<'i>(
        &'i self,
        holders: &'i [u32],
    ) -> impl Iterator<Item = &'i IndexedSoundbank> + use<'i> {
        holders
            .iter()
            .filter_map(|&position| self.soundbanks.get(usize::try_from(position).ok()?))
    }

    /// Returns the Entries that were left out, with the reason, in path order per kind.
    #[must_use]
    pub fn issues(&self) -> &[AudioIssue] {
        &self.issues
    }

    /// Returns where the soundbanks embed the WEM `id` in their `DATA` sections, in path order.
    pub fn embedded_wem(&self, id: u32) -> impl Iterator<Item = WemLocation<'_>> {
        self.media
            .get(&id)
            .map_or(&[][..], |spots| spots)
            .iter()
            .filter_map(|spot| {
                Some(WemLocation::Soundbank {
                    soundbank: self.soundbanks.get(usize::try_from(spot.soundbank).ok()?)?,
                    offset: spot.offset,
                    len: spot.len,
                })
            })
    }

    /// Returns the language folders below `Data\Sounds\wwise\` that hold soundbanks or loose WEMs,
    /// e.g. `English(US)`, in name order ignoring ASCII case.
    #[must_use]
    pub fn languages(&self) -> &[Box<str>] {
        self.loose.languages()
    }

    /// Returns the language folder called `name`, ignoring ASCII case, spelled as the Archive
    /// spells it, or `None` when the Archive holds none.
    #[must_use]
    pub fn language(&self, name: &str) -> Option<&str> {
        self.loose.language_of(name)
    }

    /// Returns the language folder `English(US)`, the game's default, or the first language folder
    /// when the Archive holds none of that name, or `None` when it holds no language folder.
    #[must_use]
    pub fn default_language(&self) -> Option<&str> {
        self.language(DEFAULT_LANGUAGE)
            .or_else(|| self.languages().first().map(|name| &**name))
    }

    /// Returns the loose WEM Entries below `Data\Sounds\wwise\` named `<id>.wem`, in path order.
    pub fn loose_wem(&self, id: u32) -> impl Iterator<Item = WemLocation<'_>> {
        self.loose.find(id).map(|(_, location)| location)
    }

    pub(crate) const fn loose(&self) -> &LooseMedia {
        &self.loose
    }

    pub(crate) fn event_holders(&self, id: u32) -> &[u32] {
        self.events.get(&id).map_or(&[], |holders| holders)
    }

    pub(crate) fn bank_positions(&self, id: u32) -> &[u32] {
        self.bank_ids.get(&id).map_or(&[], |positions| positions)
    }

    pub(crate) fn stingers(&self, trigger: u32) -> &[Stinger] {
        self.stingers.get(&trigger).map_or(&[], |found| found)
    }
}

fn boxed<T>(lists: HashMap<u32, Vec<T>>) -> HashMap<u32, Box<[T]>> {
    lists
        .into_iter()
        .map(|(id, list)| (id, list.into_boxed_slice()))
        .collect()
}

#[derive(Default)]
struct StingerList {
    found: HashMap<u32, Vec<Stinger>>,
    count: usize,
}

impl StingerList {
    fn add(&mut self, bank: u32, stingers: &[MusicStinger]) {
        for stinger in stingers {
            let found = Stinger {
                bank,
                segment_id: stinger.segment_id,
            };
            let known = self.found.entry(stinger.trigger_id).or_default();
            if self.count < MAX_STINGERS && !known.contains(&found) {
                known.push(found);
                self.count += 1;
            }
        }
    }
}

/// The segment a stinger plays and the soundbank, by position, whose music node declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stinger {
    pub(crate) bank: u32,
    pub(crate) segment_id: u32,
}

/// Where a soundbank embeds a WEM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MediaSpot {
    soundbank: u32,
    offset: u64,
    len: u64,
}
