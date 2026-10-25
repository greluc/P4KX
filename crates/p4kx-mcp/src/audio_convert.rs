// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::audio::{
    AudioIndex, BreakReason, BrokenLink, EventMatch, IndexedSoundbank, PathStep, SoundVariant,
    TriggerSounds, Unmatched, WemLocation,
};

use crate::audio_shapes::{
    BreakShape, CountsShape, EventShape, IssueShape, LocationShape, SoundbankRef, StepShape,
    TriggerEventShape, TriggerShape, UnmatchedShape, VariantShape,
};

/// The most issues one result lists.
const MAX_ISSUES: usize = 20;
/// The most soundbanks listed per event; `soundbank_count` says how many there are.
pub(crate) const MAX_BANKS: usize = 10;
/// The most events listed per trigger; `events_total` says how many there are.
pub(crate) const MAX_EVENTS: usize = 25;
const MAX_NAME_LEN: usize = crate::audio_params::MAX_NAME_LEN;

/// Returns the first issues of the audio index and how many there are.
pub(crate) fn issues(index: &AudioIndex) -> (Vec<IssueShape>, usize) {
    let listed = index
        .issues()
        .iter()
        .take(MAX_ISSUES)
        .map(|issue| IssueShape {
            path: issue.path().to_owned(),
            kind: issue.error().kind(),
            message: issue.error().to_string(),
        })
        .collect();
    (listed, index.issues().len())
}

pub(crate) fn soundbank(bank: &IndexedSoundbank) -> SoundbankRef {
    SoundbankRef {
        path: bank.path().to_owned(),
        language: bank.language().map(str::to_owned),
    }
}

/// Returns `text` cut to the name length cap, at a character boundary.
pub(crate) fn clip(text: &str) -> String {
    let end = (0..=MAX_NAME_LEN.min(text.len()))
        .rev()
        .find(|end| text.is_char_boundary(*end))
        .unwrap_or(0);
    text.get(..end).unwrap_or_default().to_owned()
}

pub(crate) fn event(event: &EventMatch<'_>, matched: Option<bool>) -> EventShape {
    EventShape {
        name: clip(event.event().name()),
        id: event.event().id(),
        localised: event.event().is_localised(),
        matched,
        soundbank_count: event.soundbanks().count(),
        soundbanks: event.soundbanks().take(MAX_BANKS).map(soundbank).collect(),
    }
}

/// Returns the first events of a trigger as shapes and how many there are.
pub(crate) fn events<'i>(
    all: impl Iterator<Item = (EventMatch<'i>, Option<bool>)>,
) -> (Vec<EventShape>, usize) {
    let mut total = 0;
    let mut listed = Vec::new();
    for (found, matched) in all {
        total += 1;
        if listed.len() < MAX_EVENTS {
            listed.push(event(&found, matched));
        }
    }
    (listed, total)
}

pub(crate) fn unmatched(reason: Unmatched) -> UnmatchedShape {
    UnmatchedShape {
        kind: reason.kind(),
        message: reason.to_string(),
    }
}

pub(crate) fn trigger(sounds: &TriggerSounds<'_>) -> TriggerShape {
    TriggerShape {
        name: clip(sounds.trigger().name()),
        definition: sounds.definition_path().map(str::to_owned),
        events_total: sounds.trigger().events().len(),
        events: sounds
            .trigger()
            .events()
            .iter()
            .take(MAX_EVENTS)
            .map(|event| TriggerEventShape {
                name: clip(event.name()),
                id: event.id(),
                localised: event.is_localised(),
            })
            .collect(),
    }
}

pub(crate) fn counts(sounds: &TriggerSounds<'_>) -> CountsShape {
    let count = |keep: fn(&SoundVariant<'_>) -> bool| {
        sounds
            .variants()
            .iter()
            .filter(|variant| keep(variant))
            .count()
    };
    CountsShape {
        variants: sounds.variants().len(),
        located: count(|variant| variant.location().is_some()),
        unlocated: count(|variant| variant.location().is_none() && variant.wem_id().is_some()),
        plugins: count(|variant| variant.wem_id().is_none()),
        breaks: sounds.breaks().len(),
    }
}

pub(crate) fn variant(variant: &SoundVariant<'_>) -> VariantShape {
    VariantShape {
        event: clip(variant.event().name()),
        soundbank: soundbank(variant.soundbank()),
        sound_id: variant.sound_id(),
        wem_id: variant.wem_id(),
        source: variant.source_kind().name(),
        localised: variant.is_localised(),
        language: variant.language().map(str::to_owned),
        fallback: variant.used_fallback(),
        location: variant.location().and_then(location),
        path: steps(variant.path()),
    }
}

pub(crate) fn broken_link(link: &BrokenLink<'_>) -> BreakShape {
    BreakShape {
        event: clip(link.event().name()),
        soundbank: link.soundbank().map(soundbank),
        object: link.object().map(step),
        target: match link.reason() {
            BreakReason::Missing { id }
            | BreakReason::OtherSoundbank { id, .. }
            | BreakReason::Cycle { id } => Some(*id),
            _ => None,
        },
        path: steps(link.path()),
        kind: link.reason().kind(),
        message: link.reason().to_string(),
    }
}

/// Returns the shape of `location`, or `None` for a kind of location this server does not know.
pub(crate) fn location(location: &WemLocation<'_>) -> Option<LocationShape> {
    match location {
        WemLocation::Soundbank {
            soundbank,
            offset,
            len,
        } => Some(LocationShape {
            kind: "soundbank",
            path: soundbank.path().to_owned(),
            language: soundbank.language().map(str::to_owned),
            offset: Some(*offset),
            len: *len,
        }),
        WemLocation::Entry {
            path,
            language,
            len,
            ..
        } => Some(LocationShape {
            kind: "entry",
            path: (*path).to_owned(),
            language: language.map(str::to_owned),
            offset: None,
            len: *len,
        }),
        _ => None,
    }
}

fn steps(path: &[PathStep]) -> Vec<StepShape> {
    path.iter().copied().map(step).collect()
}

const fn step(step: PathStep) -> StepShape {
    StepShape {
        kind: step.kind.name(),
        id: step.id,
    }
}
