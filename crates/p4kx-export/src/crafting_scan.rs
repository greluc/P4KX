// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_extraction::{CancelToken, Guid, Record, TreeBuilder};
use rayon::prelude::*;

use crate::crafting_sources;
use crate::crafting_types::{BlueprintPool, RewardSource};
use crate::{CatalogError, CatalogPhase, CatalogProgress};

const PROGRESS_STEP: usize = 1024;

/// The places that reward a pool, and how many Records that could hide one could not be read.
pub(crate) struct RewardSearch<'f> {
    pub(crate) sources: Vec<RewardSource<'f>>,
    pub(crate) unreadable: usize,
}

enum Scanned<'f> {
    Nothing,
    Found(&'f str, Vec<RewardSource<'f>>),
    Unreadable(&'f str),
}

/// Reads every Record of `records` once and returns the structs with a Reference to one of
/// `pools`, in the order of the Records.
///
/// A Record that cannot be read counts in [`RewardSearch::unreadable`] only when it has the struct
/// type of a Record in which a source was found; another type is not known to hold one.
///
/// # Errors
///
/// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
pub(crate) fn reward_sources<'f>(
    records: &[Record<'f>],
    pools: &[BlueprintPool<'f>],
    cancel: Option<&CancelToken>,
    progress: Option<&(dyn Fn(CatalogProgress) + Sync)>,
) -> Result<RewardSearch<'f>, CatalogError> {
    let stop = || cancel.is_some_and(CancelToken::is_cancelled);
    let wanted: HashSet<Guid> = pools.iter().map(|pool| pool.guid).collect();
    let done = AtomicUsize::new(0);
    let total = records.len();
    let scanned: Vec<Scanned<'f>> = records
        .par_iter()
        .map_init(TreeBuilder::new, |trees, record| {
            if stop() {
                return Err(CatalogError::Cancelled);
            }
            let scanned = scan(trees, record, &wanted);
            tick(&done, total, progress);
            Ok(scanned)
        })
        .collect::<Result<_, _>>()?;
    Ok(gather(scanned))
}

fn gather(scanned: Vec<Scanned<'_>>) -> RewardSearch<'_> {
    let mut sources = Vec::new();
    let mut holding: HashSet<&str> = HashSet::new();
    let mut unreadable: Vec<&str> = Vec::new();
    for item in scanned {
        match item {
            Scanned::Nothing => {}
            Scanned::Found(type_name, found) => {
                holding.insert(type_name);
                sources.extend(found);
            }
            Scanned::Unreadable(type_name) => unreadable.push(type_name),
        }
    }
    let unreadable = unreadable
        .into_iter()
        .filter(|type_name| holding.contains(type_name))
        .count();
    RewardSearch {
        sources,
        unreadable,
    }
}

fn scan<'f>(
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
    wanted: &HashSet<Guid>,
) -> Scanned<'f> {
    let type_name = record.type_name().unwrap_or_default();
    let Ok(tree) = trees.build(record) else {
        return Scanned::Unreadable(type_name);
    };
    let mut found = Vec::new();
    crafting_sources::collect(&tree, wanted, &mut found);
    if found.is_empty() {
        Scanned::Nothing
    } else {
        Scanned::Found(type_name, found)
    }
}

fn tick(done: &AtomicUsize, total: usize, progress: Option<&(dyn Fn(CatalogProgress) + Sync)>) {
    let finished = done.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    if let Some(progress) = progress
        && (finished.is_multiple_of(PROGRESS_STEP) || finished == total)
    {
        progress(CatalogProgress {
            phase: CatalogPhase::Crafting,
            records_done: finished,
            records_total: total,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(record: &str) -> RewardSource<'_> {
        RewardSource {
            record: Guid::default(),
            record_path: record,
            contract_id: None,
            contract_name: None,
            pool: Guid::default(),
            chance: Some(1.0),
        }
    }

    #[test]
    fn an_unreadable_record_counts_only_when_its_type_holds_a_source_elsewhere() {
        let search = gather(vec![
            Scanned::Found("ContractGenerator", vec![source("a"), source("b")]),
            Scanned::Unreadable("ContractGenerator"),
            Scanned::Unreadable("ContractGenerator"),
            Scanned::Unreadable("ShipLoadout"),
            Scanned::Nothing,
        ]);
        assert_eq!(search.unreadable, 2, "the unrelated type is not counted");
        assert_eq!(search.sources.len(), 2);
    }

    #[test]
    fn without_any_source_no_unreadable_record_counts() {
        let search = gather(vec![
            Scanned::Unreadable("ContractGenerator"),
            Scanned::Nothing,
        ]);
        assert_eq!(search.unreadable, 0);
    }

    #[test]
    fn the_sources_keep_the_order_of_the_records() {
        let search = gather(vec![
            Scanned::Found("A", vec![source("first")]),
            Scanned::Nothing,
            Scanned::Found("B", vec![source("second"), source("third")]),
        ]);
        let paths: Vec<&str> = search
            .sources
            .iter()
            .map(|found| found.record_path)
            .collect();
        assert_eq!(paths, ["first", "second", "third"]);
    }
}
