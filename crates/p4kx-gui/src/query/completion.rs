// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::query::{Completion, QuerySchema};

/// The completion of the Query editor: the names offered at the cursor and the one chosen with
/// the arrow keys, worked out again only when the text or the cursor changes.
#[derive(Debug, Default)]
pub(crate) struct CompletionState {
    shown: Option<Shown>,
    dismissed: Option<(String, usize)>,
    cursor: Option<usize>,
    waiting: Option<Trigger>,
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

    /// Returns how many bytes of every candidate name the user typed: the word before the
    /// cursor without its backticks.
    pub(crate) fn typed_len(&self) -> usize {
        self.shown.as_ref().map_or(0, |shown| {
            let start = shown.completion.replace().start;
            shown
                .text
                .get(start..shown.cursor)
                .map_or(0, |word| word.trim_matches('`').len())
        })
    }

    /// Returns a number that changes whenever the shown names do, not when only the active one
    /// moves.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// Works out the names at byte `cursor` of `text` from `schema`: opens the list after
    /// typing or when asked, also once the schema arrives if typing or asking found it missing,
    /// keeps an open list in step with the cursor, closes it where nothing fits.
    pub(crate) fn update(
        &mut self,
        schema: Option<&QuerySchema>,
        (text, cursor): (&str, usize),
        trigger: Trigger,
    ) {
        let trigger = if schema.is_none() {
            if trigger != Trigger::Moved {
                self.waiting = Some(trigger);
            }
            trigger
        } else if trigger == Trigger::Moved {
            self.waiting.take().unwrap_or(trigger)
        } else {
            self.waiting = None;
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
    }

    /// Chooses the candidate at `index` as the active one.
    pub(crate) fn choose(&mut self, index: usize) {
        if let Some(shown) = &mut self.shown
            && index < shown.completion.candidates().len()
            && shown.active != index
        {
            shown.active = index;
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

    /// Returns whether the active name is written as it is around the cursor of `text`, so that
    /// accepting it would change nothing.
    pub(crate) fn active_is_written(&self, text: &str) -> bool {
        self.shown.as_ref().is_some_and(|shown| {
            shown.text == text
                && shown
                    .completion
                    .candidates()
                    .get(shown.active)
                    .is_some_and(|candidate| {
                        text.get(shown.completion.replace()) == Some(candidate.insert())
                    })
        })
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
        self.waiting = None;
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
