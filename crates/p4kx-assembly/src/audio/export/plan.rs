// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::ExportFailure;
use super::name::{MAX_STEM_LEN, StemSet, safe_stem};
use crate::audio::{TriggerSounds, WemLocation};

const SUFFIX_RESERVE: usize = 11;

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

/// Plans the files of `triggers` in order: one per distinct WEM of a trigger, named after the
/// trigger and, where it has several, the WEM ID.
pub(super) fn build(triggers: &[&TriggerSounds<'_>]) -> Plan {
    let mut plan = Plan::default();
    let mut stems = StemSet::default();
    for sounds in triggers {
        let trigger = sounds.trigger().name();
        let mut media: Vec<(u32, Option<Origin>)> = Vec::new();
        for variant in sounds.variants() {
            let Some(wem_id) = variant.wem_id() else {
                plan.failures.push(ExportFailure {
                    trigger: trigger.to_owned(),
                    wem_id: None,
                    reason: format!(
                        "{} source {} has no file to write",
                        variant.source_kind().name(),
                        variant.source().source_id
                    ),
                });
                continue;
            };
            let found = variant.location().and_then(origin);
            match media.iter_mut().find(|(id, _)| *id == wem_id) {
                Some((_, slot)) => {
                    if slot.is_none() {
                        *slot = found;
                    }
                }
                None => media.push((wem_id, found)),
            }
        }
        let several = media.len() > 1;
        let base = safe_stem(trigger);
        for (wem_id, found) in media {
            let Some(origin) = found else {
                plan.failures.push(ExportFailure {
                    trigger: trigger.to_owned(),
                    wem_id: Some(wem_id),
                    reason: "no soundbank or loose WEM Entry holds the WEM".to_owned(),
                });
                continue;
            };
            let stem = if several {
                let kept = MAX_STEM_LEN.saturating_sub(SUFFIX_RESERVE);
                let head = base.get(..kept).unwrap_or(&base);
                format!("{head}_{wem_id}")
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
