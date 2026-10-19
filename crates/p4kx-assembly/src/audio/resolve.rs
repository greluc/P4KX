use p4kx_audio::Soundbank;
use rayon::prelude::*;

use super::read::entry_bytes;
use super::walk::{self, Found};
use super::{
    ArchiveAudio, AudioFileError, AudioIndex, BreakReason, BrokenLink, IndexedSoundbank,
    MAX_SOUNDBANK_LEN, NodeKind, PathStep, TriggerSounds, WwiseEvent,
};

type BankWalk<'i> = (u32, Result<Vec<(usize, Found<'i>)>, AudioFileError>);

impl ArchiveAudio<'_> {
    /// Resolves the audio trigger called `name`, ignoring ASCII case, to its sounds: every
    /// variant its Wwise events lead to through actions and random, sequence, switch and blend
    /// containers, and every broken link.
    ///
    /// Each soundbank holding one of the events is read again from the Archive, in parallel, and
    /// dropped once followed. Returns `None` when no ATL definition has the trigger.
    #[must_use]
    pub fn resolve_trigger(&self, name: &str) -> Option<TriggerSounds<'_>> {
        let index = self.index();
        let trigger = index.trigger(name)?;
        let events = trigger.events();
        let mut positions: Vec<u32> = events
            .iter()
            .flat_map(|event| index.event_holders(event.id()).iter().copied())
            .collect();
        positions.sort_unstable();
        positions.dedup();
        let mut walks: Vec<BankWalk<'_>> = positions
            .par_iter()
            .map(|&position| (position, self.walk_soundbank(index, position, events)))
            .collect();
        let mut sounds = TriggerSounds {
            trigger,
            definition_path: index.definition_path(trigger),
            variants: Vec::new(),
            breaks: Vec::new(),
        };
        for (number, event) in events.iter().enumerate() {
            let holders = index.event_holders(event.id());
            let event_step = Box::new([PathStep {
                kind: NodeKind::Event,
                id: event.id(),
            }]);
            if holders.is_empty() {
                sounds.breaks.push(BrokenLink {
                    event,
                    soundbank: None,
                    path: event_step,
                    reason: BreakReason::NoSoundbank,
                });
                continue;
            }
            for &position in holders {
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

    fn walk_soundbank<'i>(
        &self,
        index: &'i AudioIndex,
        position: u32,
        events: &'i [WwiseEvent],
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
            .filter(|(_, event)| index.event_holders(event.id()).contains(&position))
            .map(|(number, event)| (number, walk::event(index, &bank, soundbank, event)))
            .collect())
    }
}

fn soundbank_at(index: &AudioIndex, position: u32) -> Option<&IndexedSoundbank> {
    index.soundbanks().get(usize::try_from(position).ok()?)
}

fn gone() -> AudioFileError {
    AudioFileError::Unreadable {
        reason: "the Archive no longer holds the soundbank Entry".to_owned(),
    }
}
