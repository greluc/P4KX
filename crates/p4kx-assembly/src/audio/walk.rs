use p4kx_audio::{HircKind, HircObject, HircValue, SoundSource, Soundbank};

use super::{
    AudioIndex, BreakReason, IndexedSoundbank, LinkBreak, MAX_CONTAINER_CHILDREN,
    MAX_EVENT_OBJECTS, MAX_PATH_DEPTH, NodeKind, PathStep, SoundVariant, SourceKind, WemLocation,
    WwiseEvent,
};

/// The variants and broken links one Wwise event leads to in one soundbank.
#[derive(Debug, Default)]
pub(crate) struct Found<'i> {
    pub(crate) variants: Vec<SoundVariant<'i>>,
    pub(crate) breaks: Vec<LinkBreak<'i>>,
}

/// Follows the Wwise event `event` through the `HIRC` section of `bank`, the parsed soundbank
/// `soundbank`, to its sounds.
pub(crate) fn event<'i>(
    index: &'i AudioIndex,
    bank: &Soundbank<Vec<u8>>,
    soundbank: &'i IndexedSoundbank,
    event: &'i WwiseEvent,
) -> Found<'i> {
    let mut walk = Walk {
        index,
        bank,
        soundbank,
        event,
        path: Vec::new(),
        objects: 0,
        found: Found::default(),
    };
    walk.follow(event.id());
    walk.found
}

struct Walk<'w, 'i> {
    index: &'i AudioIndex,
    bank: &'w Soundbank<Vec<u8>>,
    soundbank: &'i IndexedSoundbank,
    event: &'i WwiseEvent,
    path: Vec<PathStep>,
    objects: usize,
    found: Found<'i>,
}

impl<'w, 'i> Walk<'w, 'i> {
    fn follow(&mut self, id: u32) {
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
        if self.objects >= MAX_EVENT_OBJECTS {
            if self.objects == MAX_EVENT_OBJECTS {
                self.objects += 1;
                self.broke(BreakReason::TooManyObjects {
                    limit: MAX_EVENT_OBJECTS,
                });
            }
            return;
        }
        self.objects += 1;
        let Some(object) = self.object(id) else {
            self.broke(BreakReason::Missing { id });
            return;
        };
        self.path.push(PathStep {
            kind: node_kind(object),
            id,
        });
        self.visit(object);
        self.path.pop();
    }

    fn object(&self, id: u32) -> Option<&'w HircObject> {
        let bank = self.bank;
        let found = bank.object(id)?;
        if self.path.is_empty() && found.kind != HircKind::Event {
            return bank
                .objects()
                .iter()
                .find(|object| object.kind == HircKind::Event && object.id == id);
        }
        Some(found)
    }

    fn visit(&mut self, object: &HircObject) {
        match &object.value {
            HircValue::Event(actions) if self.path.len() == 1 => {
                for &action in actions {
                    self.follow(action);
                }
            }
            HircValue::Action(action) => {
                if action.is_play() {
                    self.follow(action.target_id);
                }
            }
            HircValue::Container(container) => {
                for &child in container.children.iter().take(MAX_CONTAINER_CHILDREN) {
                    self.follow(child);
                }
                if container.children.len() > MAX_CONTAINER_CHILDREN {
                    self.broke(BreakReason::TooManyChildren {
                        count: container.children.len(),
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
                ) =>
            {
                self.broke(BreakReason::UnknownLayout {
                    version: self.bank.header().version,
                });
            }
            _ => self.broke(BreakReason::Unsupported),
        }
    }

    fn sound(&mut self, source: SoundSource) {
        let location = match SourceKind::of(&source) {
            SourceKind::Embedded | SourceKind::PrefetchStreamed => self.embedded(source.source_id),
            _ => None,
        };
        self.found.variants.push(SoundVariant {
            event: self.event,
            soundbank: self.soundbank,
            path: self.path.as_slice().into(),
            source,
            location,
        });
    }

    fn embedded(&self, wem: u32) -> Option<WemLocation<'i>> {
        let own = self.index.embedded_wem(wem).find(|location| {
            matches!(location, WemLocation::Soundbank { soundbank, .. }
                if std::ptr::eq(*soundbank, self.soundbank))
        });
        own.or_else(|| self.index.embedded_wem(wem).next())
    }

    fn broke(&mut self, reason: BreakReason) {
        self.found.breaks.push(LinkBreak {
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
