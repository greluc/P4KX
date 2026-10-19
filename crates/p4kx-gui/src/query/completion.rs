use std::sync::Arc;
use std::thread;

use p4kx_extraction::query::{Completion, QuerySchema};

use super::forge_slot::ForgeSlot;
use crate::EntryIndex;

/// The completion of the Query editor: the names offered at the cursor and the one chosen with
/// the arrow keys, worked out again only when the text or the cursor changes.
#[derive(Debug, Default)]
pub(crate) struct CompletionState {
    shown: Option<Shown>,
    dismissed: Option<(String, usize)>,
    cursor: Option<usize>,
    schema_seen: bool,
    generation: u64,
}

#[derive(Debug)]
struct Shown {
    text: String,
    cursor: usize,
    completion: Completion,
    active: usize,
}

/// What the editor asks of the completion this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Trigger {
    /// The text changed by typing.
    Typed,
    /// Ctrl+Space asked for the names at the cursor.
    Asked,
    /// Only the cursor may have moved.
    Moved,
}

impl CompletionState {
    /// Returns the shown completion and the index of its active candidate.
    pub(crate) fn shown(&self) -> Option<(&Completion, usize)> {
        self.shown
            .as_ref()
            .map(|shown| (&shown.completion, shown.active))
    }

    /// Returns a number that changes whenever the shown completion or its active candidate does.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// Works out the names at byte `cursor` of `text` from `schema`: opens the list after
    /// typing, when asked or once the schema arrives, keeps an open list in step with the
    /// cursor, closes it where nothing fits.
    pub(crate) fn update(
        &mut self,
        schema: Option<&QuerySchema>,
        (text, cursor): (&str, usize),
        trigger: Trigger,
    ) {
        let arrived = schema.is_some() && !self.schema_seen;
        self.schema_seen = schema.is_some();
        let trigger = if arrived && trigger == Trigger::Moved {
            Trigger::Typed
        } else {
            trigger
        };
        if trigger == Trigger::Moved && self.cursor == Some(cursor) {
            return;
        }
        self.cursor = Some(cursor);
        let opening = match trigger {
            Trigger::Typed => self
                .dismissed
                .as_ref()
                .is_none_or(|(dismissed, at)| dismissed != text || *at != cursor),
            Trigger::Asked => true,
            Trigger::Moved => self.shown.is_some(),
        };
        if trigger == Trigger::Asked {
            self.dismissed = None;
        }
        let completion = opening
            .then(|| schema.and_then(|schema| schema.complete(text, cursor)))
            .flatten();
        self.set(completion.map(|completion| Shown {
            text: text.to_owned(),
            cursor,
            completion,
            active: 0,
        }));
    }

    /// Moves the active candidate by `steps`, wrapping around.
    pub(crate) fn step(&mut self, steps: isize) {
        let Some(shown) = &mut self.shown else {
            return;
        };
        let len = shown.completion.candidates().len();
        if len == 0 {
            return;
        }
        let len = isize::try_from(len).unwrap_or(isize::MAX);
        let active = isize::try_from(shown.active).unwrap_or(0);
        shown.active = usize::try_from((active + steps).rem_euclid(len)).unwrap_or(0);
        self.generation = self.generation.wrapping_add(1);
    }

    /// Chooses the candidate at `index` as the active one.
    pub(crate) fn choose(&mut self, index: usize) {
        if let Some(shown) = &mut self.shown
            && index < shown.completion.candidates().len()
            && shown.active != index
        {
            shown.active = index;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// Writes the active candidate into `text`; returns the byte offset behind it, or `None`
    /// when no list is shown or `text` is no longer the text it was made for.
    pub(crate) fn accept(&mut self, text: &mut String) -> Option<usize> {
        let shown = self.shown.take()?;
        self.generation = self.generation.wrapping_add(1);
        if shown.text != *text {
            return None;
        }
        let candidate = shown.completion.candidates().get(shown.active)?;
        let replace = shown.completion.replace();
        text.get(replace.clone())?;
        text.replace_range(replace.clone(), candidate.insert());
        Some(replace.start + candidate.insert().len())
    }

    /// Closes the list until the text or the cursor changes or it is asked for again.
    pub(crate) fn dismiss(&mut self) {
        if let Some(shown) = self.shown.take() {
            self.dismissed = Some((shown.text, shown.cursor));
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// Forgets everything, because another Archive was opened.
    pub(crate) fn reset(&mut self) {
        self.close();
        let generation = self.generation;
        *self = Self {
            generation,
            ..Self::default()
        };
    }

    /// Closes the list.
    pub(crate) fn close(&mut self) {
        self.set(None);
    }

    fn set(&mut self, shown: Option<Shown>) {
        if self.shown.is_none() && shown.is_none() {
            return;
        }
        self.shown = shown;
        self.generation = self.generation.wrapping_add(1);
    }
}

/// Reads the completion schema of `index` into `slot` on a background thread unless it was
/// asked for before, calling `wake` once it is read.
pub(crate) fn prepare(
    slot: &Arc<ForgeSlot>,
    index: &Arc<EntryIndex>,
    wake: impl Fn() + Send + 'static,
) {
    if !slot.ask_schema() {
        return;
    }
    let slot = Arc::clone(slot);
    let index = Arc::clone(index);
    let spawned = thread::Builder::new()
        .name("query-schema".to_owned())
        .spawn(move || {
            if slot.read_schema(&index).is_ok() {
                wake();
            }
        });
    drop(spawned);
}
