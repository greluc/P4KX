use std::cmp::Ordering;

use rayon::prelude::*;

use crate::path_key::cmp_normalised;
use crate::{
    Archive, CancelToken, CompareOptions, Comparison, ComparisonError, ComparisonProgress, Entry,
    EntryChange, Side,
};

const PROGRESS_STEP: usize = 1 << 16;

/// Compares the `old` with the `new` Archive on the selected levels.
///
/// Entries are joined by their path with ASCII case folded and `/` read as `\`; an Entry is
/// changed when its uncompressed size or CRC-32C differs. Nothing is decompressed.
///
/// # Errors
///
/// Returns [`ComparisonError::NoLevel`] when no level is selected and
/// [`ComparisonError::Cancelled`] when the Comparison was cancelled.
pub fn compare<'a>(
    old: &'a Archive,
    new: &'a Archive,
    options: &CompareOptions<'_>,
) -> Result<Comparison<'a>, ComparisonError> {
    if options.levels.is_empty() {
        return Err(ComparisonError::NoLevel);
    }
    check_cancel(options)?;
    let entries = if options.levels.entries {
        compare_entries(old, new, options)?
    } else {
        Vec::new()
    };
    Ok(Comparison {
        old: Side::of(old),
        new: Side::of(new),
        levels: options.levels,
        filter: options.filter.map(|filter| filter.as_str().to_owned()),
        entries,
    })
}

fn compare_entries<'a>(
    old: &'a Archive,
    new: &'a Archive,
    options: &CompareOptions<'_>,
) -> Result<Vec<EntryChange<'a>>, ComparisonError> {
    let (old, new) = rayon::join(
        || sorted_selection(old, options),
        || sorted_selection(new, options),
    );
    check_cancel(options)?;
    let total = old.len() + new.len();
    let report = |done| {
        if let Some(progress) = options.progress {
            progress(ComparisonProgress {
                entries_done: done,
                entries_total: total,
            });
        }
    };
    report(0);
    let mut changes = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut next_report = PROGRESS_STEP;
    loop {
        let change = match (old.get(i), new.get(j)) {
            (None, None) => break,
            (Some(&before), None) => {
                i += 1;
                Some(EntryChange::removed(before))
            }
            (None, Some(&after)) => {
                j += 1;
                Some(EntryChange::added(after))
            }
            (Some(&before), Some(&after)) => match cmp_normalised(before.path(), after.path()) {
                Ordering::Less => {
                    i += 1;
                    Some(EntryChange::removed(before))
                }
                Ordering::Greater => {
                    j += 1;
                    Some(EntryChange::added(after))
                }
                Ordering::Equal => {
                    i += 1;
                    j += 1;
                    differs(before, after).then(|| EntryChange::changed(before, after))
                }
            },
        };
        changes.extend(change);
        if i + j >= next_report {
            check_cancel(options)?;
            report(i + j);
            next_report = i + j + PROGRESS_STEP;
        }
    }
    report(total);
    Ok(changes)
}

fn sorted_selection<'a>(archive: &'a Archive, options: &CompareOptions<'_>) -> Vec<&'a Entry> {
    let mut entries: Vec<&Entry> = options.filter.map_or_else(
        || archive.entries().iter().collect(),
        |filter| {
            archive
                .entries()
                .par_iter()
                .filter(|entry| filter.matches(entry.path()))
                .collect()
        },
    );
    entries.par_sort_by(|a, b| {
        cmp_normalised(a.path(), b.path()).then_with(|| a.path().cmp(b.path()))
    });
    entries
}

const fn differs(old: &Entry, new: &Entry) -> bool {
    old.uncompressed_size() != new.uncompressed_size() || old.crc32c() != new.crc32c()
}

fn check_cancel(options: &CompareOptions<'_>) -> Result<(), ComparisonError> {
    if options.cancel.is_some_and(CancelToken::is_cancelled) {
        Err(ComparisonError::Cancelled)
    } else {
        Ok(())
    }
}
