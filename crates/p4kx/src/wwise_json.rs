use p4kx_assembly::audio::{
    BreakReason, BrokenLink, IndexedSoundbank, PathStep, SoundVariant, TriggerSounds, WemLocation,
    WwiseEvent,
};
use serde::Serialize;

use crate::entity_audio::IssueJson;

/// The JSON shape of `p4kx wwise resolve`.
#[derive(Serialize)]
pub(crate) struct ResolveJson<'a> {
    pub(crate) archive: String,
    pub(crate) trigger: TriggerJson<'a>,
    pub(crate) variants: Vec<VariantJson<'a>>,
    pub(crate) breaks: Vec<BreakJson<'a>>,
    pub(crate) counts: Counts,
    pub(crate) issues: Vec<IssueJson<'a>>,
}

/// The JSON shape of a resolved trigger.
#[derive(Serialize)]
pub(crate) struct TriggerJson<'a> {
    name: &'a str,
    definition: Option<&'a str>,
    events: Vec<EventJson<'a>>,
}

#[derive(Serialize)]
struct EventJson<'a> {
    name: &'a str,
    id: u32,
    localised: bool,
}

#[derive(Serialize)]
pub(crate) struct VariantJson<'a> {
    event: &'a str,
    soundbank: SoundbankJson<'a>,
    sound_id: u32,
    wem_id: Option<u32>,
    source: &'static str,
    location: Option<LocationJson<'a>>,
    path: Vec<StepJson>,
}

#[derive(Serialize)]
struct SoundbankJson<'a> {
    path: &'a str,
    language: Option<&'a str>,
}

#[derive(Serialize)]
struct LocationJson<'a> {
    kind: &'static str,
    soundbank: &'a str,
    offset: u64,
    len: u64,
}

#[derive(Serialize)]
struct StepJson {
    kind: &'static str,
    id: u32,
}

#[derive(Serialize)]
pub(crate) struct BreakJson<'a> {
    event: &'a str,
    soundbank: Option<SoundbankJson<'a>>,
    object: Option<StepJson>,
    target: Option<u32>,
    path: Vec<StepJson>,
    kind: &'static str,
    message: String,
}

/// How many variants a trigger has, how many of them are located and how many links break.
#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct Counts {
    pub(crate) variants: usize,
    pub(crate) located: usize,
    pub(crate) unlocated: usize,
    pub(crate) plugins: usize,
    pub(crate) breaks: usize,
}

impl Counts {
    pub(crate) fn of(sounds: &TriggerSounds<'_>) -> Self {
        let count = |keep: fn(&SoundVariant<'_>) -> bool| {
            sounds
                .variants()
                .iter()
                .filter(|variant| keep(variant))
                .count()
        };
        Self {
            variants: sounds.variants().len(),
            located: count(|variant| variant.location().is_some()),
            unlocated: count(|variant| variant.location().is_none() && variant.wem_id().is_some()),
            plugins: count(|variant| variant.wem_id().is_none()),
            breaks: sounds.breaks().len(),
        }
    }
}

pub(crate) fn trigger<'a>(sounds: &TriggerSounds<'a>) -> TriggerJson<'a> {
    TriggerJson {
        name: sounds.trigger().name(),
        definition: sounds.definition_path(),
        events: sounds.trigger().events().iter().map(event).collect(),
    }
}

fn event(event: &WwiseEvent) -> EventJson<'_> {
    EventJson {
        name: event.name(),
        id: event.id(),
        localised: event.is_localised(),
    }
}

pub(crate) fn variant<'a>(variant: &SoundVariant<'a>) -> VariantJson<'a> {
    VariantJson {
        event: variant.event().name(),
        soundbank: soundbank(variant.soundbank()),
        sound_id: variant.sound_id(),
        wem_id: variant.wem_id(),
        source: variant.source_kind().name(),
        location: variant.location().map(location),
        path: steps(variant.path()),
    }
}

pub(crate) fn broken_link<'a>(link: &BrokenLink<'a>) -> BreakJson<'a> {
    BreakJson {
        event: link.event().name(),
        soundbank: link.soundbank().map(soundbank),
        object: link.object().map(step),
        target: match link.reason() {
            BreakReason::Missing { id } | BreakReason::Cycle { id } => Some(*id),
            _ => None,
        },
        path: steps(link.path()),
        kind: link.reason().kind(),
        message: link.reason().to_string(),
    }
}

fn soundbank(bank: &IndexedSoundbank) -> SoundbankJson<'_> {
    SoundbankJson {
        path: bank.path(),
        language: bank.language(),
    }
}

fn location<'a>(location: &WemLocation<'a>) -> LocationJson<'a> {
    match location {
        WemLocation::Soundbank {
            soundbank,
            offset,
            len,
        } => LocationJson {
            kind: "soundbank",
            soundbank: soundbank.path(),
            offset: *offset,
            len: *len,
        },
        _ => LocationJson {
            kind: "other",
            soundbank: "",
            offset: 0,
            len: 0,
        },
    }
}

fn steps(path: &[PathStep]) -> Vec<StepJson> {
    path.iter().copied().map(step).collect()
}

const fn step(step: PathStep) -> StepJson {
    StepJson {
        kind: step.kind.name(),
        id: step.id,
    }
}
