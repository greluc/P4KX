// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_audio::Soundbank;
use rayon::prelude::*;

use super::read::entry_bytes;
use super::walk::{self, Found};
use super::{
    ArchiveAudio, AudioFileError, AudioIndex, BreakReason, BrokenLink, IndexedSoundbank,
    LanguageChoice, MAX_EVENT_OBJECTS, MAX_SOUNDBANK_LEN, MAX_TRIGGER_OBJECTS, NodeKind, PathStep,
    TriggerSounds, WwiseEvent,
};

type BankWalk<'i> = (u32, Result<Vec<(usize, Found<'i>)>, AudioFileError>);

impl ArchiveAudio<'_> {
    /// Resolves the audio trigger called `name`, ignoring ASCII case, to its sounds in every
    /// language: see [`resolve_trigger_in`](Self::resolve_trigger_in).
    #[must_use]
    pub fn resolve_trigger(&self, name: &str) -> Option<TriggerSounds<'_>> {
        self.resolve_trigger_in(name, &LanguageChoice::any())
    }

    /// Resolves the audio trigger called `name`, ignoring ASCII case, to its sounds: every
    /// variant its Wwise events lead to through actions and random, sequence, switch and blend
    /// containers, and every broken link.
    ///
    /// A localised event is followed only in the soundbanks of the language `choice` names, or of
    /// its fallback language when the first has none; a streamed localised sound is located in
    /// that language's folder of loose WEMs. Where neither language has the sound, the trigger
    /// gets a [`BreakReason::NoLocalisation`] break.
    ///
    /// Each soundbank holding one of the events is read again from the Archive, in parallel, and
    /// dropped once followed. Returns `None` when no ATL definition has the trigger.
    #[must_use]
    pub fn resolve_trigger_in(
        &self,
        name: &str,
        choice: &LanguageChoice,
    ) -> Option<TriggerSounds<'_>> {
        let index = self.index();
        let trigger = index.trigger(name)?;
        let events = trigger.events();
        let holders: Vec<Vec<u32>> = events
            .iter()
            .map(|event| select_holders(index, index.event_holders(event.id()), choice))
            .collect();
        let mut walks = self.walk_soundbanks(index, events, &holders, choice);
        let mut sounds = TriggerSounds {
            trigger,
            definition_path: index.definition_path(trigger),
            variants: Vec::new(),
            breaks: Vec::new(),
        };
        for (number, event) in events.iter().enumerate() {
            let event_step = Box::new([PathStep {
                kind: NodeKind::Event,
                id: event.id(),
            }]);
            let selected = holders.get(number).map_or(&[][..], Vec::as_slice);
            if selected.is_empty() {
                sounds
                    .breaks
                    .push(unselected(index, event, choice, event_step));
                continue;
            }
            for &position in selected {
                let Some(soundbank) = soundbank_at(index, position) else {
                    continue;
                };
                let Ok(slot) = walks.binary_search_by_key(&position, |(at, _)| *at) else {
                    continue;
                };
                match walks.get_mut(slot).map(|(_, walk)| walk) {
                    Some(Ok(found)) => {
                        if let Some((_, found)) = found.iter_mut().find(|(at, _)| *at == number) {
                            sounds.variants.append(&mut found.variants);
                            sounds.breaks.append(&mut found.breaks);
                        }
                    }
                    Some(Err(error)) => sounds.breaks.push(BrokenLink {
                        event,
                        soundbank: Some(soundbank),
                        path: event_step.clone(),
                        reason: BreakReason::Unreadable(error.clone()),
                    }),
                    None => {}
                }
            }
        }
        Some(sounds)
    }

    fn walk_soundbanks<'i>(
        &self,
        index: &'i AudioIndex,
        events: &'i [WwiseEvent],
        holders: &[Vec<u32>],
        choice: &LanguageChoice,
    ) -> Vec<BankWalk<'i>> {
        let mut positions: Vec<u32> = holders.iter().flatten().copied().collect();
        positions.sort_unstable();
        positions.dedup();
        let pairs: usize = holders.iter().map(Vec::len).sum();
        let budget = (MAX_TRIGGER_OBJECTS / pairs.max(1)).clamp(1, MAX_EVENT_OBJECTS);
        positions
            .par_iter()
            .map(|&position| {
                (
                    position,
                    self.walk_soundbank(index, position, events, holders, choice, budget),
                )
            })
            .collect()
    }

    fn walk_soundbank<'i>(
        &self,
        index: &'i AudioIndex,
        position: u32,
        events: &'i [WwiseEvent],
        holders: &[Vec<u32>],
        choice: &LanguageChoice,
        budget: usize,
    ) -> Result<Vec<(usize, Found<'i>)>, AudioFileError> {
        let soundbank = soundbank_at(index, position).ok_or_else(gone)?;
        let entry = self
            .archive()
            .entries()
            .get(soundbank.entry)
            .ok_or_else(gone)?;
        let bank = Soundbank::parse(entry_bytes(self.archive(), entry, MAX_SOUNDBANK_LEN)?)?;
        Ok(events
            .iter()
            .enumerate()
            .filter(|(number, _)| {
                holders
                    .get(*number)
                    .is_some_and(|selected| selected.contains(&position))
            })
            .map(|(number, event)| {
                (
                    number,
                    walk::event(index, &bank, soundbank, event, choice, budget),
                )
            })
            .collect())
    }
}

fn unselected<'i>(
    index: &AudioIndex,
    event: &'i WwiseEvent,
    choice: &LanguageChoice,
    path: Box<[PathStep]>,
) -> BrokenLink<'i> {
    let reason = if index.event_holders(event.id()).is_empty() {
        BreakReason::NoSoundbank
    } else {
        choice.no_localisation()
    };
    BrokenLink {
        event,
        soundbank: None,
        path,
        reason,
    }
}

/// Returns the soundbanks of `holders` an event is followed in: every soundbank that is not
/// localised, and of the localised ones those of the first language of `choice` that has any.
fn select_holders(index: &AudioIndex, holders: &[u32], choice: &LanguageChoice) -> Vec<u32> {
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

fn soundbank_at(index: &AudioIndex, position: u32) -> Option<&IndexedSoundbank> {
    index.soundbanks().get(usize::try_from(position).ok()?)
}

fn gone() -> AudioFileError {
    AudioFileError::Unreadable {
        reason: "the Archive no longer holds the soundbank Entry".to_owned(),
    }
}
