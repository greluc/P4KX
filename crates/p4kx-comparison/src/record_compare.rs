use std::cmp::Ordering;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use rayon::prelude::*;

use crate::compare::check_cancel;
use crate::field_diff::diff_fields;
use crate::path_key::cmp_normalised;
use crate::{
    CompareOptions, ComparisonError, ComparisonProgress, DataForge, DataForgeError, DataForges,
    Record, RecordChange, RecordChangeKind, RecordState,
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
    progress_so_far: ComparisonProgress,
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
                ..progress_so_far
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
    let (old, new) = match pair {
        Joined::Removed(old) => (Some(old), None),
        Joined::Added(new) => (None, Some(new)),
        Joined::Both(old, new) => (Some(old), Some(new)),
    };
    let old_path = old.map(|record| path(record, OLD)).transpose()?;
    let new_path = new.map(|record| path(record, NEW)).transpose()?;
    if let Some(filter) = options.filter
        && !old_path
            .into_iter()
            .chain(new_path)
            .any(|path| filter.matches(path))
    {
        return Ok(None);
    }
    let guid = new.or(old).map(|record| record.guid()).unwrap_or_default();
    let mut change = RecordChange {
        guid,
        old: old
            .zip(old_path)
            .map(|(record, path)| state(record, path, OLD))
            .transpose()?,
        new: new
            .zip(new_path)
            .map(|(record, path)| state(record, path, NEW))
            .transpose()?,
        fields: Vec::new(),
    };
    let (Some(old), Some(new), Some(before), Some(after)) = (old, new, change.old, change.new)
    else {
        return Ok(Some(change));
    };
    let old_tree = old.tree().map_err(|source| unreadable(old, OLD, source))?;
    let new_tree = new.tree().map_err(|source| unreadable(new, NEW, source))?;
    change.fields = diff_fields(old_tree.fields(), new_tree.fields());
    let differs = !change.fields.is_empty()
        || before.name != after.name
        || before.type_name != after.type_name
        || change.kind() == RecordChangeKind::Moved;
    Ok(differs.then_some(change))
}

fn path<'a>(record: Record<'a>, side: &'static str) -> Result<&'a str, ComparisonError> {
    record
        .record_path()
        .map_err(|source| unreadable(record, side, source))
}

fn state<'a>(
    record: Record<'a>,
    record_path: &'a str,
    side: &'static str,
) -> Result<RecordState<'a>, ComparisonError> {
    let read = || {
        Ok::<_, DataForgeError>(RecordState {
            record_path,
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
