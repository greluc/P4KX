use std::cmp::Ordering;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use rayon::prelude::*;

use crate::compare::check_cancel;
use crate::field_diff::diff_fields;
use crate::path_key::{cmp_normalised, eq_normalised};
use crate::{
    CompareOptions, ComparisonError, ComparisonProgress, DataForge, DataForgeError, DataForges,
    Record, RecordChange, RecordState,
};

const PROGRESS_STEP: usize = 1 << 12;
const OLD: &str = "old";
const NEW: &str = "new";

/// The Records of one GUID: on one side only, or on both.
#[derive(Clone, Copy)]
enum Joined<'a> {
    Removed(Record<'a>),
    Added(Record<'a>),
    Both(Record<'a>, Record<'a>),
}

/// Joins the Records of both DataForges by GUID and returns the added, removed, moved and
/// changed ones, ordered by record path ignoring case and separator kind, then by GUID.
pub(crate) fn compare_records<'a>(
    forges: DataForges<'a>,
    options: &CompareOptions<'_>,
    entries: ComparisonProgress,
) -> Result<Vec<RecordChange<'a>>, ComparisonError> {
    let (old, new) = rayon::join(|| by_guid(forges.old), || by_guid(forges.new));
    check_cancel(options)?;
    let joined = join(&old, &new);
    drop((old, new));
    let total = joined
        .iter()
        .filter(|pair| matches!(pair, Joined::Both(..)))
        .count();
    let done = AtomicUsize::new(0);
    let report = |done| {
        if let Some(progress) = options.progress {
            progress(ComparisonProgress {
                records_done: done,
                records_total: total,
                ..entries
            });
        }
    };
    report(0);
    let changes: Result<Vec<Option<RecordChange<'a>>>, ComparisonError> = joined
        .into_par_iter()
        .map(|pair| {
            check_cancel(options)?;
            let change = diff(pair, options);
            if matches!(pair, Joined::Both(..)) {
                let finished = done.fetch_add(1, AtomicOrdering::Relaxed).saturating_add(1);
                if finished.is_multiple_of(PROGRESS_STEP) {
                    report(finished);
                }
            }
            change
        })
        .collect();
    let mut changes: Vec<RecordChange<'a>> = changes?.into_iter().flatten().collect();
    changes.par_sort_unstable_by(|a, b| {
        cmp_normalised(a.path(), b.path())
            .then_with(|| a.path().cmp(b.path()))
            .then_with(|| a.guid().cmp(&b.guid()))
    });
    report(total);
    Ok(changes)
}

fn by_guid<'a>(forge: &'a DataForge<'a>) -> Vec<Record<'a>> {
    let mut records: Vec<Record<'a>> = forge.records().collect();
    records.par_sort_unstable_by_key(|record| (record.guid(), record.index()));
    records
}

fn join<'a>(old: &[Record<'a>], new: &[Record<'a>]) -> Vec<Joined<'a>> {
    let mut joined = Vec::with_capacity(old.len().max(new.len()));
    let (mut i, mut j) = (0, 0);
    loop {
        let pair = match (old.get(i), new.get(j)) {
            (None, None) => break,
            (Some(&before), None) => {
                i += 1;
                Joined::Removed(before)
            }
            (None, Some(&after)) => {
                j += 1;
                Joined::Added(after)
            }
            (Some(&before), Some(&after)) => match before.guid().cmp(&after.guid()) {
                Ordering::Less => {
                    i += 1;
                    Joined::Removed(before)
                }
                Ordering::Greater => {
                    j += 1;
                    Joined::Added(after)
                }
                Ordering::Equal => {
                    i += 1;
                    j += 1;
                    Joined::Both(before, after)
                }
            },
        };
        joined.push(pair);
    }
    joined
}

fn diff<'a>(
    pair: Joined<'a>,
    options: &CompareOptions<'_>,
) -> Result<Option<RecordChange<'a>>, ComparisonError> {
    let selected = |paths: &[&str]| {
        options
            .filter
            .is_none_or(|filter| paths.iter().any(|path| filter.matches(path)))
    };
    match pair {
        Joined::Removed(old) => {
            let state = state(old, OLD)?;
            Ok(selected(&[state.record_path]).then(|| RecordChange {
                guid: old.guid(),
                old: Some(state),
                new: None,
                fields: Vec::new(),
            }))
        }
        Joined::Added(new) => {
            let state = state(new, NEW)?;
            Ok(selected(&[state.record_path]).then(|| RecordChange {
                guid: new.guid(),
                old: None,
                new: Some(state),
                fields: Vec::new(),
            }))
        }
        Joined::Both(old, new) => {
            let (before, after) = (state(old, OLD)?, state(new, NEW)?);
            if !selected(&[before.record_path, after.record_path]) {
                return Ok(None);
            }
            let old_tree = old.tree().map_err(|source| unreadable(old, OLD, source))?;
            let new_tree = new.tree().map_err(|source| unreadable(new, NEW, source))?;
            let fields = diff_fields(old_tree.fields(), new_tree.fields());
            let differs = !fields.is_empty()
                || before.name != after.name
                || before.type_name != after.type_name
                || !eq_normalised(before.record_path, after.record_path);
            Ok(differs.then(|| RecordChange {
                guid: new.guid(),
                old: Some(before),
                new: Some(after),
                fields,
            }))
        }
    }
}

fn state<'a>(record: Record<'a>, side: &'static str) -> Result<RecordState<'a>, ComparisonError> {
    let read = || {
        Ok::<_, DataForgeError>(RecordState {
            record_path: record.record_path()?,
            name: record.name()?,
            type_name: record.type_name()?,
        })
    };
    read().map_err(|source| unreadable(record, side, source))
}

const fn unreadable(
    record: Record<'_>,
    side: &'static str,
    source: DataForgeError,
) -> ComparisonError {
    ComparisonError::Record {
        side,
        guid: record.guid(),
        source,
    }
}
