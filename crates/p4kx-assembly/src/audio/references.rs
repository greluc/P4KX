// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_dataforge::{Location, RecordTree, Step, StructValue, Value};

/// The struct through which a Record names an audio trigger.
const AUDIO_STRUCT: &str = "GlobalResourceAudio";

/// The field of [`AUDIO_STRUCT`] that holds the trigger name.
const TRIGGER_FIELD: &str = "audioTrigger";

/// An audio trigger a Record names: the trigger name, where the Record names it first, and how
/// often.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerReference {
    name: Box<str>,
    location: Box<str>,
    count: u32,
}

impl TriggerReference {
    /// Returns the trigger name as the Record first writes it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the location of the first field naming it, e.g.
    /// `Components[3].triggers[0].audioTrigger`.
    #[must_use]
    pub fn location(&self) -> &str {
        &self.location
    }

    /// Returns how many fields of the Record name it.
    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }
}

/// Which fields of a Record may name an audio trigger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TriggerScope {
    /// Only the `audioTrigger` field of a `GlobalResourceAudio` struct.
    GlobalResourceAudio,
    /// The `audioTrigger` field of a `GlobalResourceAudio` struct and every string field of any
    /// other struct whose name contains `audioTrigger`, ignoring ASCII case, e.g. the
    /// `audioTriggerName` of a melee weapon.
    #[default]
    AudioTriggerFields,
}

/// Returns the audio triggers a Record tree names through [`TriggerScope::AudioTriggerFields`]:
/// see [`trigger_references_in`].
#[must_use]
pub fn trigger_references(tree: &RecordTree<'_>) -> Vec<TriggerReference> {
    trigger_references_in(tree, TriggerScope::AudioTriggerFields)
}

/// Returns the audio triggers a Record tree names through the structs `scope` allows.
///
/// Each trigger is listed once, in the order it first appears, names matched ignoring ASCII
/// case; empty trigger names are skipped.
#[must_use]
pub fn trigger_references_in(tree: &RecordTree<'_>, scope: TriggerScope) -> Vec<TriggerReference> {
    let mut walk = Walk {
        scope,
        ..Walk::default()
    };
    walk.structure(tree.root());
    walk.found
}

fn names_a_trigger(field: &str) -> bool {
    field
        .as_bytes()
        .windows(TRIGGER_FIELD.len())
        .any(|window| window.eq_ignore_ascii_case(TRIGGER_FIELD.as_bytes()))
}

#[derive(Default)]
struct Walk<'f> {
    scope: TriggerScope,
    steps: Vec<Step<'f>>,
    found: Vec<TriggerReference>,
    seen: HashMap<Box<str>, usize>,
}

impl<'f> Walk<'f> {
    fn structure(&mut self, structure: &StructValue<'f>) {
        let audio = structure.type_name() == AUDIO_STRUCT;
        if audio
            && let Some(Value::String(name)) = structure.field(TRIGGER_FIELD)
            && !name.is_empty()
        {
            self.steps.push(Step::Field(TRIGGER_FIELD));
            self.add(name);
            self.steps.pop();
        }
        let anywhere = self.scope == TriggerScope::AudioTriggerFields && !audio;
        for field in structure.fields() {
            self.steps.push(Step::Field(field.name()));
            if anywhere
                && names_a_trigger(field.name())
                && let Value::String(name) = field.value()
                && !name.is_empty()
            {
                self.add(name);
            }
            self.value(field.value());
            self.steps.pop();
        }
    }

    fn value(&mut self, value: &Value<'f>) {
        match value {
            Value::Struct(structure) => self.structure(structure),
            Value::Array(items) => {
                for (position, item) in (0_u32..).zip(items.iter()) {
                    if matches!(item, Value::Struct(_) | Value::Array(_)) {
                        self.steps.push(Step::Element(position));
                        self.value(item);
                        self.steps.pop();
                    }
                }
            }
            _ => {}
        }
    }

    fn add(&mut self, name: &str) {
        let key: Box<str> = name.to_ascii_lowercase().into();
        if let Some(reference) = self
            .seen
            .get(&key)
            .and_then(|&position| self.found.get_mut(position))
        {
            reference.count = reference.count.saturating_add(1);
            return;
        }
        self.seen.insert(key, self.found.len());
        self.found.push(TriggerReference {
            name: name.into(),
            location: Location::new(&self.steps).as_str().into(),
            count: 1,
        });
    }
}
