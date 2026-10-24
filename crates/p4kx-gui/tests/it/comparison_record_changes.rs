// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record list of the Comparison tab over a pair of synthetic DataForges: change kinds,
//! record paths, types, sorting and the Filter.

use crate::record_model;
use crate::record_pair;

use p4kx_comparison::RecordChangeKind;
use p4kx_gui::{
    ComparisonLevels, ComparisonList, ComparisonView, RecordChangeColumn, RecordChangeView,
    SortDirection,
};
use record_model::{compared, records, records_mut, wait};
use record_pair::{BOTH, TestResult, pair};

type Row = (RecordChangeKind, Option<String>, Option<String>, String);

fn row(kind: RecordChangeKind, old: Option<&str>, new: Option<&str>, type_name: &str) -> Row {
    (
        kind,
        old.map(str::to_owned),
        new.map(str::to_owned),
        type_name.to_owned(),
    )
}

fn rows(records: &RecordChangeView) -> Vec<Row> {
    records
        .shown()
        .iter()
        .filter_map(|&id| records.row(id))
        .map(|row| {
            (
                row.kind(),
                row.old_path().map(str::to_owned),
                row.new_path().map(str::to_owned),
                row.type_name().to_owned(),
            )
        })
        .collect()
}

fn paths(records: &RecordChangeView) -> Vec<String> {
    records
        .shown()
        .iter()
        .filter_map(|&id| records.row(id))
        .map(|row| row.path().to_owned())
        .collect()
}

fn expected_rows() -> Vec<Row> {
    vec![
        row(
            RecordChangeKind::Unreadable,
            Some("libs/broken/one"),
            Some("libs/broken/one"),
            "Broken",
        ),
        row(
            RecordChangeKind::Changed,
            Some("libs/items/sample"),
            Some("libs/items/sample"),
            "Item",
        ),
        row(
            RecordChangeKind::Removed,
            Some("libs/makers/gone"),
            None,
            "Maker",
        ),
        row(
            RecordChangeKind::Added,
            None,
            Some("libs/ships/fresh"),
            "Ship",
        ),
        row(
            RecordChangeKind::Moved,
            Some("libs/ships/wanderer"),
            Some("libs/ships/moved/wanderer"),
            "Ship",
        ),
        row(
            RecordChangeKind::Changed,
            Some("libs/ships/renamed"),
            Some("libs/ships/renamed"),
            "Ship",
        ),
        row(
            RecordChangeKind::Changed,
            Some("libs/ships/retyped"),
            Some("libs/ships/retyped"),
            "Maker",
        ),
        row(
            RecordChangeKind::Changed,
            Some("libs/ships/tuned"),
            Some("libs/ships/tuned"),
            "Ship",
        ),
    ]
}

#[test]
fn the_record_list_shows_kind_paths_and_type_in_path_order() -> TestResult {
    let (_dir, old, new) = pair()?;

    let model = compared((&old, &new), BOTH)?;

    let records = records(&model)?;
    assert_eq!(
        rows(records),
        expected_rows(),
        "every kind of change, the moved Record with both paths, the type of the new side"
    );
    assert_eq!(
        model.comparison().view().map(ComparisonView::list),
        Some(ComparisonList::Entries),
        "the Entries come first while they were compared"
    );
    Ok(())
}

#[test]
fn the_record_list_sorts_by_change_and_by_type() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    records_mut(&mut model)?.sort_by(RecordChangeColumn::Change);
    let by_change = paths(records(&model)?);
    records_mut(&mut model)?.sort_by(RecordChangeColumn::Change);
    let descending = records(&model)?.sort().direction;
    let by_change_reversed = paths(records(&model)?);
    records_mut(&mut model)?.sort_by(RecordChangeColumn::Type);
    let by_type = paths(records(&model)?);

    assert_eq!(
        by_change,
        [
            "libs/ships/fresh",
            "libs/makers/gone",
            "libs/ships/moved/wanderer",
            "libs/items/sample",
            "libs/ships/renamed",
            "libs/ships/retyped",
            "libs/ships/tuned",
            "libs/broken/one",
        ],
        "added, removed, moved, changed, unreadable; ties in path order"
    );
    assert_eq!(
        descending,
        SortDirection::Descending,
        "a second click reverses"
    );
    assert_eq!(
        by_change_reversed.first().map(String::as_str),
        Some("libs/broken/one"),
        "unreadable first when descending"
    );
    assert_eq!(
        by_type,
        [
            "libs/broken/one",
            "libs/items/sample",
            "libs/makers/gone",
            "libs/ships/retyped",
            "libs/ships/fresh",
            "libs/ships/moved/wanderer",
            "libs/ships/renamed",
            "libs/ships/tuned",
        ],
        "Broken, Item, Maker, then the Ships in path order"
    );
    Ok(())
}

#[test]
fn the_filter_selects_records_by_either_record_path() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let view = model.comparison_mut().view_mut().ok_or("no view")?;
    view.set_filter("ships/wanderer", || {});
    wait(model.comparison_mut(), |tab| {
        tab.view()
            .and_then(ComparisonView::records)
            .is_some_and(|records| !records.is_filtering())
    })?;
    let by_old_path = paths(records(&model)?);
    let view = model.comparison_mut().view_mut().ok_or("no view")?;
    view.set_filter("", || {});
    let all = records(&model)?.shown().len();

    assert_eq!(
        by_old_path,
        ["libs/ships/moved/wanderer"],
        "the moved Record matches by its old path"
    );
    assert_eq!(all, 8, "no Filter, every Record");
    Ok(())
}

#[test]
fn the_record_level_alone_shows_the_record_list_first() -> TestResult {
    let (_dir, old, new) = pair()?;
    let records = ComparisonLevels {
        entries: false,
        records: true,
    };

    let model = compared((&old, &new), records)?;

    let view = model.comparison().view().ok_or("no view")?;
    assert_eq!(view.list(), ComparisonList::Records, "the Record list");
    assert_eq!(view.shown_len(), 8, "its rows are counted");
    Ok(())
}

#[test]
fn without_the_record_level_there_is_no_record_list() -> TestResult {
    let (_dir, old, new) = pair()?;

    let mut model = compared((&old, &new), ComparisonLevels::default())?;

    let view = model.comparison_mut().view_mut().ok_or("no view")?;
    view.show_list(ComparisonList::Records);
    assert!(view.records().is_none(), "no Record list");
    assert_eq!(view.list(), ComparisonList::Entries, "the Entries stay");
    Ok(())
}
