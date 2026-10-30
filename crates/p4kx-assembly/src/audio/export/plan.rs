// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::ExportFailure;
use super::name::{StemSet, portable_stem};
use crate::audio::{SoundVariant, SourceKind, TriggerSounds, WemLocation};

/// The stem of a sound whose trigger name is too long for a file name.
const FALLBACK_STEM: &str = "sound";

/// Where the bytes of a WEM are: an Entry of the Archive, by position in `Archive::entries`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Origin {
    Soundbank(usize),
    Loose(usize),
}

/// One sound to write.
#[derive(Debug)]
pub(super) struct Job {
    pub(super) trigger: String,
    pub(super) stem: String,
    pub(super) wem_id: u32,
    pub(super) origin: Origin,
}

/// The sounds to write, with their file names fixed, and the sounds that have no WEM to write.
#[derive(Debug, Default)]
pub(super) struct Plan {
    pub(super) jobs: Vec<Job>,
    pub(super) failures: Vec<ExportFailure>,
}

fn origin(location: &WemLocation<'_>) -> Option<Origin> {
    match *location {
        WemLocation::Soundbank { soundbank, .. } => Some(Origin::Soundbank(soundbank.entry)),
        WemLocation::Entry { entry, .. } => usize::try_from(entry).ok().map(Origin::Loose),
    }
}

#[derive(Debug, Clone, Copy)]
struct Found {
    origin: Origin,
    prefetch_only: bool,
}

fn prefetch_only(variant: &SoundVariant<'_>) -> bool {
    variant.source_kind() == SourceKind::PrefetchStreamed
        && matches!(variant.location(), Some(WemLocation::Soundbank { .. }))
}

const fn better(candidate: Option<Found>, current: Option<Found>) -> bool {
    match (candidate, current) {
        (Some(_), None) => true,
        (Some(candidate), Some(current)) => current.prefetch_only && !candidate.prefetch_only,
        (None, _) => false,
    }
}

/// Plans the files of `triggers` in order: one per distinct WEM of a trigger, named after the
/// trigger and, where it has several, the WEM ID.
pub(super) fn build(triggers: &[&TriggerSounds<'_>]) -> Plan {
    let mut plan = Plan::default();
    let mut stems = StemSet::default();
    for sounds in triggers {
        let trigger = sounds.trigger().name();
        let mut media: Vec<(u32, Option<Found>)> = Vec::new();
        for variant in sounds.variants() {
            let Some(wem_id) = variant.wem_id() else {
                plan.failures.push(ExportFailure {
                    trigger: trigger.to_owned(),
                    wem_id: None,
                    reason: format!(
                        "{} {} has no file to write",
                        variant.source_kind().label(),
                        variant.source().source_id
                    ),
                });
                continue;
            };
            let found = variant.location().and_then(origin).map(|origin| Found {
                origin,
                prefetch_only: prefetch_only(variant),
            });
            match media.iter_mut().find(|(id, _)| *id == wem_id) {
                Some((_, slot)) => {
                    if better(found, *slot) {
                        *slot = found;
                    }
                }
                None => media.push((wem_id, found)),
            }
        }
        let several = media.len() > 1;
        let base = portable_stem(trigger, FALLBACK_STEM);
        for (wem_id, found) in media {
            let Some(Found { origin, .. }) = found else {
                plan.failures.push(ExportFailure {
                    trigger: trigger.to_owned(),
                    wem_id: Some(wem_id),
                    reason: "no soundbank or loose WEM Entry holds the WEM".to_owned(),
                });
                continue;
            };
            let stem = if several {
                format!("{base}_{wem_id}")
            } else {
                base.clone()
            };
            plan.jobs.push(Job {
                trigger: trigger.to_owned(),
                stem: stems.unique(&stem),
                wem_id,
                origin,
            });
        }
    }
    plan
}
