use p4kx_dataforge::{DataForgeError, Record};
use thiserror::Error;

use super::{
    AtlTrigger, AudioIndex, IndexedSoundbank, TriggerReference, WwiseEvent, trigger_references,
};

/// Why an audio trigger an Entity names leads to no soundbank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Unmatched {
    /// No ATL definition has the trigger.
    #[error("no ATL definition has the trigger")]
    NoDefinition,
    /// The trigger's ATL definition posts no Wwise event.
    #[error("the ATL definition posts no Wwise event")]
    NoEvent,
    /// No soundbank holds any of the trigger's Wwise events.
    #[error("no soundbank holds the Wwise event")]
    NoSoundbank,
}

impl Unmatched {
    /// Returns a stable code for the reason, e.g. `no_definition`, for scripts.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::NoDefinition => "no_definition",
            Self::NoEvent => "no_event",
            Self::NoSoundbank => "no_soundbank",
        }
    }
}

/// A Wwise event of a trigger with the soundbanks that hold it.
#[derive(Debug, Clone, Copy)]
pub struct EventMatch<'i> {
    event: &'i WwiseEvent,
    holders: &'i [u32],
    index: &'i AudioIndex,
}

impl<'i> EventMatch<'i> {
    /// Returns the event.
    #[must_use]
    pub const fn event(&self) -> &'i WwiseEvent {
        self.event
    }

    pub(crate) fn of_trigger(
        index: &'i AudioIndex,
        trigger: &'i AtlTrigger,
    ) -> impl Iterator<Item = Self> + use<'i> {
        trigger.events().iter().map(move |event| Self {
            event,
            holders: index.event_holders(event.id()),
            index,
        })
    }

    /// Returns the soundbanks whose `HIRC` section holds the event, in path order.
    pub fn soundbanks(&self) -> impl Iterator<Item = &'i IndexedSoundbank> + use<'i> {
        self.index.soundbanks_at(self.holders)
    }

    /// Returns whether a soundbank holds the event.
    #[must_use]
    pub const fn is_matched(&self) -> bool {
        !self.holders.is_empty()
    }
}

/// An audio trigger an Entity names, with its ATL definition when there is one.
#[derive(Debug, Clone)]
pub struct EntityTrigger<'i> {
    reference: TriggerReference,
    definition: Option<&'i AtlTrigger>,
    index: &'i AudioIndex,
}

impl<'i> EntityTrigger<'i> {
    /// Returns where and how often the Entity names the trigger.
    #[must_use]
    pub const fn reference(&self) -> &TriggerReference {
        &self.reference
    }

    /// Returns the trigger name as the Entity writes it.
    #[must_use]
    pub fn name(&self) -> &str {
        self.reference.name()
    }

    /// Returns the ATL definition of the trigger, or `None` when no definition has it.
    #[must_use]
    pub const fn definition(&self) -> Option<&'i AtlTrigger> {
        self.definition
    }

    /// Returns the path of the ATL definition Entry, or `None` when no definition has it.
    #[must_use]
    pub fn definition_path(&self) -> Option<&'i str> {
        self.definition
            .and_then(|definition| self.index.definition_path(definition))
    }

    /// Returns the Wwise events the trigger posts, each with the soundbanks that hold it.
    pub fn events(&self) -> impl Iterator<Item = EventMatch<'i>> + use<'i> {
        let index = self.index;
        self.definition
            .into_iter()
            .flat_map(move |definition| EventMatch::of_trigger(index, definition))
    }

    /// Returns why the trigger leads to no soundbank, or `None` when a soundbank holds one of its
    /// events.
    #[must_use]
    pub fn unmatched(&self) -> Option<Unmatched> {
        let Some(definition) = self.definition else {
            return Some(Unmatched::NoDefinition);
        };
        unmatched_of(self.index, definition)
    }
}

pub(crate) fn unmatched_of(index: &AudioIndex, definition: &AtlTrigger) -> Option<Unmatched> {
    if definition.events().is_empty() {
        return Some(Unmatched::NoEvent);
    }
    (!EventMatch::of_trigger(index, definition).any(|event| event.is_matched()))
        .then_some(Unmatched::NoSoundbank)
}

/// The audio triggers one Entity names, resolved against an [`AudioIndex`].
#[derive(Debug, Clone)]
pub struct EntityAudio<'i> {
    triggers: Vec<EntityTrigger<'i>>,
}

impl<'i> EntityAudio<'i> {
    /// Returns the triggers in the order the Entity first names them.
    #[must_use]
    pub fn triggers(&self) -> &[EntityTrigger<'i>] {
        &self.triggers
    }

    /// Returns how many triggers lead to a soundbank.
    #[must_use]
    pub fn matched_count(&self) -> usize {
        self.triggers
            .iter()
            .filter(|trigger| trigger.unmatched().is_none())
            .count()
    }
}

impl AudioIndex {
    /// Reads the audio triggers the Record of an Entity names and resolves each to its ATL
    /// definition, its Wwise events and the soundbanks holding them.
    ///
    /// # Errors
    ///
    /// Returns the [`DataForgeError`] of [`Record::tree`] when the Record cannot be read.
    pub fn entity_audio(&self, record: &Record<'_>) -> Result<EntityAudio<'_>, DataForgeError> {
        let tree = record.tree()?;
        Ok(self.resolve(trigger_references(&tree)))
    }

    /// Resolves trigger references against the index.
    #[must_use]
    pub fn resolve(&self, references: Vec<TriggerReference>) -> EntityAudio<'_> {
        EntityAudio {
            triggers: references
                .into_iter()
                .map(|reference| EntityTrigger {
                    definition: self.trigger(reference.name()),
                    reference,
                    index: self,
                })
                .collect(),
        }
    }
}
