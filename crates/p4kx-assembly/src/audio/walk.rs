// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_audio::{HircKind, HircObject, HircValue, PlayBank, SoundSource, Soundbank};

use super::locate::locate;
use super::{
    AudioIndex, BreakReason, BrokenLink, IndexedSoundbank, LanguageChoice, MAX_CONTAINER_CHILDREN,
    MAX_PATH_DEPTH, NodeKind, PathStep, SoundVariant, WwiseEvent,
};

/// The variants and broken links one Wwise event leads to in one soundbank.
#[derive(Debug, Default)]
pub(crate) struct Found<'i> {
    pub(crate) variants: Vec<SoundVariant<'i>>,
    pub(crate) breaks: Vec<BrokenLink<'i>>,
}

/// Follows the Wwise event `event` through the `HIRC` section of `bank`, the parsed soundbank
/// `soundbank`, to its sounds.
pub(crate) fn event<'i>(
    index: &'i AudioIndex,
    bank: &Soundbank<Vec<u8>>,
    soundbank: &'i IndexedSoundbank,
    event: &'i WwiseEvent,
    choice: &LanguageChoice,
    budget: usize,
) -> Found<'i> {
    let mut walk = Walk {
        index,
        choice,
        bank,
        soundbank,
        event,
        path: Vec::new(),
        objects: 0,
        budget,
        exhausted: false,
        found: Found::default(),
    };
    walk.follow(event.id());
    walk.found
}

struct Walk<'w, 'i> {
    index: &'i AudioIndex,
    choice: &'w LanguageChoice,
    bank: &'w Soundbank<Vec<u8>>,
    soundbank: &'i IndexedSoundbank,
    event: &'i WwiseEvent,
    path: Vec<PathStep>,
    objects: usize,
    budget: usize,
    exhausted: bool,
    found: Found<'i>,
}

impl<'w> Walk<'w, '_> {
    fn follow(&mut self, id: u32) {
        self.reach(id, None, None);
    }

    fn spend(&mut self) -> bool {
        if self.objects >= self.budget {
            if !self.exhausted {
                self.exhausted = true;
                self.broke(BreakReason::TooManyObjects { limit: self.budget });
            }
            return false;
        }
        self.objects += 1;
        true
    }

    fn reach(&mut self, id: u32, wanted: Option<HircKind>, bank: Option<PlayBank>) {
        if !self.spend() {
            return;
        }
        if self.path.len() >= MAX_PATH_DEPTH {
            self.broke(BreakReason::TooDeep {
                limit: MAX_PATH_DEPTH,
            });
            return;
        }
        if self.path.iter().any(|step| step.id == id) {
            self.broke(BreakReason::Cycle { id });
            return;
        }
        let posted = wanted.is_some();
        let wanted = wanted.or_else(|| self.path.is_empty().then_some(HircKind::Event));
        let Some(object) = self.object(id, wanted) else {
            let own = self.bank.header().id;
            let holder = if posted {
                self.index
                    .event_soundbanks(id)
                    .map(IndexedSoundbank::id)
                    .find(|&holder| holder != own)
            } else {
                bank.map(|bank| bank.bank_id)
                    .filter(|&named| named != 0 && named != own)
            };
            let reason = holder.map_or(BreakReason::Missing { id }, |bank_id| {
                BreakReason::OtherSoundbank { id, bank_id }
            });
            self.broke(reason);
            return;
        };
        self.path.push(PathStep {
            kind: node_kind(object),
            id,
        });
        self.visit(object);
        self.path.pop();
    }

    fn object(&self, id: u32, wanted: Option<HircKind>) -> Option<&'w HircObject> {
        let bank = self.bank;
        let found = bank.object(id)?;
        match wanted {
            Some(kind) if found.kind != kind => bank
                .objects()
                .iter()
                .find(|object| object.kind == kind && object.id == id),
            _ => Some(found),
        }
    }

    fn visit(&mut self, object: &HircObject) {
        match &object.value {
            HircValue::Event(actions) => self.children(actions),
            HircValue::Action(action) => {
                if action.is_play() {
                    self.reach(action.target_id, None, action.bank);
                } else if action.is_play_event() {
                    self.reach(action.target_id, Some(HircKind::Event), None);
                }
            }
            HircValue::Container(container) => self.children(&container.children),
            HircValue::MusicSegment(segment) => self.children(&segment.node.children),
            HircValue::MusicSwitch(switch) => self.children(&switch.node.children),
            HircValue::MusicPlaylist(playlist) => self.children(&playlist.node.children),
            HircValue::MusicTrack(track) => {
                for source in track.sources.iter().take(MAX_CONTAINER_CHILDREN) {
                    if !self.spend() {
                        break;
                    }
                    self.sound(*source);
                }
                if track.sources.len() > MAX_CONTAINER_CHILDREN {
                    self.broke(BreakReason::TooManyChildren {
                        count: track.sources.len(),
                        limit: MAX_CONTAINER_CHILDREN,
                    });
                }
            }
            HircValue::Sound(source) => self.sound(*source),
            HircValue::Malformed(error) => self.broke(BreakReason::Malformed(error.clone())),
            HircValue::Raw
                if matches!(
                    object.kind,
                    HircKind::Sound
                        | HircKind::RandomSequenceContainer
                        | HircKind::SwitchContainer
                        | HircKind::LayerContainer
                        | HircKind::MusicSegment
                        | HircKind::MusicTrack
                        | HircKind::MusicSwitch
                        | HircKind::MusicPlaylist
                ) =>
            {
                self.broke(BreakReason::UnknownLayout {
                    version: self.bank.header().version,
                });
            }
            HircValue::Raw => self.broke(BreakReason::Unsupported),
        }
    }

    fn children(&mut self, ids: &[u32]) {
        for &id in ids.iter().take(MAX_CONTAINER_CHILDREN) {
            self.follow(id);
        }
        if ids.len() > MAX_CONTAINER_CHILDREN {
            self.broke(BreakReason::TooManyChildren {
                count: ids.len(),
                limit: MAX_CONTAINER_CHILDREN,
            });
        }
    }

    fn sound(&mut self, source: SoundSource) {
        let located = locate(self.index, self.soundbank, self.choice, &source);
        if located.missing_localisation {
            self.broke(self.choice.no_localisation());
        }
        self.found.variants.push(SoundVariant {
            event: self.event,
            soundbank: self.soundbank,
            sound_id: self.path.last().map_or(0, |step| step.id),
            path: self.path.as_slice().into(),
            source,
            location: located.location,
            localised: located.localised,
            language: located.language,
            fallback: located.fallback,
        });
    }

    fn broke(&mut self, reason: BreakReason) {
        self.found.breaks.push(BrokenLink {
            event: self.event,
            soundbank: Some(self.soundbank),
            path: self.path.as_slice().into(),
            reason,
        });
    }
}

const fn node_kind(object: &HircObject) -> NodeKind {
    match (&object.value, object.kind) {
        (HircValue::Container(container), _) => NodeKind::of_container(container.kind),
        (_, HircKind::Event) => NodeKind::Event,
        (_, HircKind::Action) => NodeKind::Action,
        (_, HircKind::Sound) => NodeKind::Sound,
        (_, kind) => NodeKind::Other(kind),
    }
}
