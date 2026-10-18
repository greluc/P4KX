use std::collections::HashMap;
use std::collections::hash_map::Entry as Slot;

use p4kx_archive::{Archive, Entry};
use rayon::prelude::*;

use super::read::{entry_bytes, folded_order, has_extension, starts_with_folder};
use super::{
    ATL_FOLDER, AtlTrigger, AudioFileError, AudioIssue, IndexedSoundbank, MAX_AUDIO_TRIGGERS, atl,
    banks,
};

type BankRead = Result<(IndexedSoundbank, Vec<u32>), AudioFileError>;

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
        let mut bank_entries: Vec<&Entry> = archive
            .entries()
            .iter()
            .filter(|entry| has_extension(entry.path(), "bnk"))
            .collect();
        atl_entries.sort_by(|a, b| folded_order(a.path(), b.path()));
        bank_entries.sort_by(|a, b| folded_order(a.path(), b.path()));
        let definitions: Vec<_> = atl_entries
            .par_iter()
            .enumerate()
            .map(|(position, entry)| {
                let file = u32::try_from(position).unwrap_or(u32::MAX);
                entry_bytes(archive, entry).and_then(|bytes| atl::parse(&bytes, file))
            })
            .collect();
        let soundbanks: Vec<BankRead> = bank_entries
            .par_iter()
            .map(|entry| {
                entry_bytes(archive, entry).and_then(|bytes| banks::read(entry.path(), &bytes))
            })
            .collect();
        let mut index = Self::default();
        index.add_definitions(&atl_entries, definitions);
        index.add_soundbanks(&bank_entries, soundbanks);
        index
    }

    fn add_definitions(
        &mut self,
        entries: &[&Entry],
        definitions: Vec<Result<Vec<AtlTrigger>, AudioFileError>>,
    ) {
        self.atl_files = entries.iter().map(|entry| entry.path().into()).collect();
        for (entry, definition) in entries.iter().zip(definitions) {
            let triggers = match definition {
                Ok(triggers)
                    if self.triggers.len().saturating_add(triggers.len()) > MAX_AUDIO_TRIGGERS =>
                {
                    Err(AudioFileError::TooManyTriggers)
                }
                other => other,
            };
            match triggers {
                Ok(triggers) => {
                    for trigger in triggers {
                        let position = self.triggers.len();
                        if let Slot::Vacant(slot) =
                            self.by_name.entry(trigger.name.to_ascii_lowercase().into())
                        {
                            slot.insert(position);
                            self.triggers.push(trigger);
                        }
                    }
                }
                Err(error) => self.issues.push(AudioIssue {
                    path: entry.path().into(),
                    error,
                }),
            }
        }
    }

    fn add_soundbanks(&mut self, entries: &[&Entry], soundbanks: Vec<BankRead>) {
        let mut events: HashMap<u32, Vec<u32>> = HashMap::new();
        for (entry, soundbank) in entries.iter().zip(soundbanks) {
            match soundbank {
                Ok((soundbank, ids)) => {
                    let position = u32::try_from(self.soundbanks.len()).unwrap_or(u32::MAX);
                    self.soundbanks.push(soundbank);
                    for id in ids {
                        let holders = events.entry(id).or_default();
                        if holders.last() != Some(&position) {
                            holders.push(position);
                        }
                    }
                }
                Err(error) => self.issues.push(AudioIssue {
                    path: entry.path().into(),
                    error,
                }),
            }
        }
        self.events = events
            .into_iter()
            .map(|(id, holders)| (id, holders.into_boxed_slice()))
            .collect();
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

    /// Returns the path of the ATL definition Entry that defines `trigger`.
    #[must_use]
    pub fn definition_path(&self, trigger: &AtlTrigger) -> &str {
        usize::try_from(trigger.file)
            .ok()
            .and_then(|file| self.atl_files.get(file))
            .map_or("", |path| path)
    }

    /// Returns the paths of the ATL definition Entries that were read or tried, in path order.
    #[must_use]
    pub fn definition_paths(&self) -> &[Box<str>] {
        &self.atl_files
    }

    /// Returns every soundbank that was parsed, in path order.
    #[must_use]
    pub fn soundbanks(&self) -> &[IndexedSoundbank] {
        &self.soundbanks
    }

    /// Returns the soundbanks whose `HIRC` section holds the Event object `id`, in path order.
    pub fn event_soundbanks(&self, id: u32) -> impl Iterator<Item = &IndexedSoundbank> {
        self.event_holders(id)
            .iter()
            .filter_map(|&position| self.soundbanks.get(usize::try_from(position).ok()?))
    }

    /// Returns the Entries that were left out, with the reason, in path order per kind.
    #[must_use]
    pub fn issues(&self) -> &[AudioIssue] {
        &self.issues
    }

    pub(crate) fn event_holders(&self, id: u32) -> &[u32] {
        self.events.get(&id).map_or(&[], |holders| holders)
    }
}
