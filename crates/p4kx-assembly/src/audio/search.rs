// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_foundation::PathPattern;

use super::entity::unmatched_of;
use super::{AtlTrigger, AudioIndex, EventMatch, Unmatched};

/// The most hits one page of a search holds.
pub const MAX_SEARCH_PAGE: usize = 10_000;

/// An audio trigger a search found, with the part of its names that matched.
#[derive(Clone)]
pub struct TriggerHit<'i> {
    trigger: &'i AtlTrigger,
    index: &'i AudioIndex,
    pattern: Arc<PathPattern>,
    by_trigger: bool,
}

impl std::fmt::Debug for TriggerHit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerHit")
            .field("trigger", &self.trigger)
            .field("by_trigger", &self.by_trigger)
            .finish_non_exhaustive()
    }
}

impl<'i> TriggerHit<'i> {
    /// Returns the trigger as its ATL definition writes it.
    #[must_use]
    pub const fn trigger(&self) -> &'i AtlTrigger {
        self.trigger
    }

    /// Returns the path of the ATL definition Entry that defines the trigger.
    #[must_use]
    pub fn definition_path(&self) -> Option<&'i str> {
        self.index.definition_path(self.trigger)
    }

    /// Returns whether the trigger name matched.
    #[must_use]
    pub const fn matched_by_trigger(&self) -> bool {
        self.by_trigger
    }

    /// Returns every Wwise event the trigger posts with the soundbanks that hold it, and whether
    /// the event name matched.
    pub fn events(&self) -> impl Iterator<Item = (EventMatch<'i>, bool)> + use<'i> {
        let pattern = Arc::clone(&self.pattern);
        EventMatch::of_trigger(self.index, self.trigger)
            .map(move |event| (event, pattern.matches(event.event().name())))
    }

    /// Returns why the trigger leads to no soundbank, or `None` when a soundbank holds one of its
    /// events.
    #[must_use]
    pub fn unmatched(&self) -> Option<Unmatched> {
        unmatched_of(self.index, self.trigger)
    }
}

/// One page of the triggers a search found.
#[derive(Debug, Clone)]
pub struct SearchPage<'i> {
    hits: Vec<TriggerHit<'i>>,
    total: usize,
    offset: usize,
}

impl<'i> SearchPage<'i> {
    /// Returns the hits of the page, in the order of the ATL definitions.
    #[must_use]
    pub fn hits(&self) -> &[TriggerHit<'i>] {
        &self.hits
    }

    /// Returns how many triggers matched in all.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }

    /// Returns how many hits precede the page.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the offset of the next page, or `None` when the page reaches the last hit.
    #[must_use]
    pub fn next_offset(&self) -> Option<usize> {
        let end = self.offset.saturating_add(self.hits.len());
        (end < self.total).then_some(end)
    }
}

impl AudioIndex {
    /// Searches the trigger names and the names of the Wwise events they post for `text`
    /// (a substring, or a glob with `*` and `?`, ignoring ASCII case) and returns the hits from
    /// position `offset` on, at most `limit` of them (and at most [`MAX_SEARCH_PAGE`]).
    ///
    /// The hits are in the order of [`triggers`](Self::triggers), so the same search gives the
    /// same pages. Events that no ATL definition names cannot be found by name.
    #[must_use]
    pub fn search(&self, text: &str, offset: usize, limit: usize) -> SearchPage<'_> {
        let pattern = Arc::new(PathPattern::new(text));
        let limit = limit.min(MAX_SEARCH_PAGE);
        let mut hits = Vec::with_capacity(limit.min(self.triggers().len()));
        let mut total = 0_usize;
        for trigger in self.triggers() {
            let by_trigger = pattern.matches(trigger.name());
            if !by_trigger
                && !trigger
                    .events()
                    .iter()
                    .any(|event| pattern.matches(event.name()))
            {
                continue;
            }
            if total >= offset && hits.len() < limit {
                hits.push(TriggerHit {
                    trigger,
                    index: self,
                    pattern: Arc::clone(&pattern),
                    by_trigger,
                });
            }
            total += 1;
        }
        SearchPage {
            hits,
            total,
            offset,
        }
    }
}
