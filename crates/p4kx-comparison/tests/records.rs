//! The Record level of a Comparison through the public API: joining Records by GUID.

mod forge_pair;

use std::error::Error;
use std::sync::Mutex;

use forge_pair::ForgePair;
use p4kx_comparison::{
    CancelToken, CompareOptions, ComparisonError, ComparisonProgress, Filter, Levels,
    RecordChangeKind, compare,
};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const SAME: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0001;
const GONE: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0002;
const MOVED: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0003;
const TUNED: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0004;
const ADDED: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0005;

fn ship() -> TestStruct {
    TestStruct::new("Ship")
        .field("speed", TestType::Int32)
        .field("mass", TestType::Single)
}

fn record(guid: u128, name: &str, path: &str, speed: i32) -> TestRecord {
    TestRecord::new(guid, "Ship", format!("Ship.{name}"), path)
        .value("speed", TestValue::Int32(speed))
        .value("mass", TestValue::Single(1.5))
}

fn database(records: impl IntoIterator<Item = TestRecord>) -> DataForgeBuilder {
    records.into_iter().fold(
        DataForgeBuilder::new().structure(ship()),
        DataForgeBuilder::record,
    )
}

fn pair() -> Result<ForgePair, Box<dyn Error>> {
    ForgePair::new(
        &database([
            record(SAME, "Same", "libs/ships/same.xml", 1),
            record(GONE, "Gone", "libs/ships/gone.xml", 2),
            record(MOVED, "Moved", "libs/ships/old_home.xml", 3),
            record(TUNED, "Tuned", "libs/ships/tuned.xml", 4),
        ]),
        &database([
            record(ADDED, "Added", "libs/ships/added.xml", 5),
            record(TUNED, "Tuned", "libs/ships/tuned.xml", 40),
            record(MOVED, "Moved", "libs/ships/new_home.xml", 3),
            record(SAME, "Same", "libs/ships/same.xml", 1),
        ]),
    )
}

#[test]
fn records_are_joined_by_guid_and_unchanged_ones_left_out() -> TestResult {
    let pair = pair()?;

    let changes = pair.compare_records(|comparison| {
        comparison
            .record_changes()
            .iter()
            .map(|change| {
                (
                    change.kind(),
                    change.guid().to_string(),
                    change.path().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    })?;

    assert_eq!(
        changes,
        [
            (
                RecordChangeKind::Added,
                "10000000-0000-4000-8000-000000000005".to_owned(),
                "libs/ships/added".to_owned()
            ),
            (
                RecordChangeKind::Removed,
                "10000000-0000-4000-8000-000000000002".to_owned(),
                "libs/ships/gone".to_owned()
            ),
            (
                RecordChangeKind::Moved,
                "10000000-0000-4000-8000-000000000003".to_owned(),
                "libs/ships/new_home".to_owned()
            ),
            (
                RecordChangeKind::Changed,
                "10000000-0000-4000-8000-000000000004".to_owned(),
                "libs/ships/tuned".to_owned()
            ),
        ],
        "ordered by record path, unchanged Records left out"
    );
    Ok(())
}

#[test]
fn a_moved_record_keeps_both_record_paths_and_is_not_removed_and_added() -> TestResult {
    let pair = pair()?;

    let (old, new, fields) = pair
        .compare_records(|comparison| {
            let moved = comparison
                .record_changes()
                .iter()
                .find(|change| change.kind() == RecordChangeKind::Moved);
            moved.map(|change| {
                (
                    change
                        .old_record()
                        .map(|side| side.record_path().to_owned()),
                    change
                        .new_record()
                        .map(|side| side.record_path().to_owned()),
                    change.fields().len(),
                )
            })
        })?
        .ok_or("a moved Record")?;

    assert_eq!(
        old.as_deref(),
        Some("libs/ships/old_home"),
        "old record path"
    );
    assert_eq!(
        new.as_deref(),
        Some("libs/ships/new_home"),
        "new record path"
    );
    assert_eq!(fields, 0, "no field changed");
    Ok(())
}

#[test]
fn a_changed_record_lists_the_changed_field_with_old_and_new_value() -> TestResult {
    let pair = pair()?;

    let fields = pair
        .compare_records(|comparison| {
            comparison
                .record_changes()
                .iter()
                .find(|change| change.kind() == RecordChangeKind::Changed)
                .map(|change| {
                    change
                        .fields()
                        .iter()
                        .map(|field| {
                            (
                                field.path().to_string(),
                                field.old_value().cloned(),
                                field.new_value().cloned(),
                            )
                        })
                        .map(|(path, old, new)| format!("{path}: {old:?} -> {new:?}"))
                        .collect::<Vec<_>>()
                })
        })?
        .ok_or("a changed Record")?;

    assert_eq!(
        fields,
        ["speed: Some(Value(Int32(4))) -> Some(Value(Int32(40)))"],
        "only the changed field"
    );
    Ok(())
}

#[test]
fn records_are_counted_per_change_kind() -> TestResult {
    let pair = pair()?;

    let (counts, differs) = pair
        .compare_records(|comparison| (comparison.record_counts(), comparison.has_differences()))?;

    assert_eq!(
        (counts.added, counts.removed, counts.moved, counts.changed),
        (1, 1, 1, 1),
        "one of each"
    );
    assert_eq!(counts.total(), 4, "total");
    assert!(differs, "Record changes are differences");
    Ok(())
}

#[test]
fn identical_dataforges_have_no_record_differences() -> TestResult {
    let records = [record(SAME, "Same", "libs/ships/same.xml", 1)];
    let pair = ForgePair::new(&database(records.clone()), &database(records))?;

    let (changes, differs) = pair.compare_records(|comparison| {
        (
            comparison.record_changes().len(),
            comparison.has_differences(),
        )
    })?;

    assert_eq!(changes, 0, "no Record changed");
    assert!(!differs, "no differences");
    Ok(())
}

#[test]
fn the_filter_selects_records_by_either_record_path() -> TestResult {
    let pair = pair()?;
    let filter = Filter::new("*home*");

    let paths = pair.compare_with(
        &CompareOptions {
            levels: Levels::RECORDS,
            filter: Some(&filter),
            ..CompareOptions::default()
        },
        |comparison| {
            comparison
                .record_changes()
                .iter()
                .map(|change| change.path().to_owned())
                .collect::<Vec<_>>()
        },
    )?;

    assert_eq!(paths, ["libs/ships/new_home"], "only the moved Record");
    Ok(())
}

#[test]
fn the_record_level_needs_both_dataforges() -> TestResult {
    let pair = pair()?;

    let result = compare(
        &pair.old,
        &pair.new,
        None,
        &CompareOptions {
            levels: Levels::RECORDS,
            ..CompareOptions::default()
        },
    );

    assert!(
        matches!(result, Err(ComparisonError::NoDataForge)),
        "no DataForge: {result:?}"
    );
    Ok(())
}

#[test]
fn levels_select_entries_records_or_both() -> TestResult {
    let pair = pair()?;

    for (levels, entries, records) in [
        (Levels::ENTRIES, true, false),
        (Levels::RECORDS, false, true),
        (Levels::ALL, true, true),
    ] {
        let compared = pair.compare_with(
            &CompareOptions {
                levels,
                ..CompareOptions::default()
            },
            |comparison| {
                (
                    comparison.levels().entries,
                    comparison.levels().records,
                    comparison.record_changes().len(),
                )
            },
        )?;
        assert_eq!(
            compared,
            (entries, records, if records { 4 } else { 0 }),
            "levels {levels:?}"
        );
    }
    Ok(())
}

#[test]
fn record_progress_ends_with_every_joined_record_done() -> TestResult {
    let pair = pair()?;
    let seen = Mutex::new(Vec::new());
    let record = |progress: ComparisonProgress| {
        if let Ok(mut seen) = seen.lock() {
            seen.push(progress);
        }
    };

    pair.compare_with(
        &CompareOptions {
            levels: Levels::RECORDS,
            progress: Some(&record),
            ..CompareOptions::default()
        },
        |_| (),
    )?;

    let seen = seen.into_inner()?;
    let last = seen.last().ok_or("progress was reported")?;
    assert_eq!(
        (last.records_done, last.records_total),
        (3, 3),
        "the three Records both sides have"
    );
    Ok(())
}

#[test]
fn a_cancelled_record_comparison_returns_cancelled() -> TestResult {
    let pair = pair()?;
    let cancel = CancelToken::new();
    cancel.cancel();

    let result = pair.compare_with(
        &CompareOptions {
            levels: Levels::RECORDS,
            cancel: Some(&cancel),
            ..CompareOptions::default()
        },
        |_| (),
    );

    assert!(
        result
            .err()
            .and_then(|error| error.downcast::<ComparisonError>().ok())
            .is_some_and(|error| *error == ComparisonError::Cancelled),
        "cancelled"
    );
    Ok(())
}

#[test]
fn a_renamed_record_at_the_same_path_is_changed() -> TestResult {
    let pair = ForgePair::new(
        &database([record(SAME, "Before", "libs/ships/same.xml", 1)]),
        &database([record(SAME, "After", "libs/ships/same.xml", 1)]),
    )?;

    let change = pair.compare_records(|comparison| {
        comparison.record_changes().first().map(|change| {
            (
                change.kind(),
                change.old_record().map(|side| side.name().to_owned()),
                change.new_record().map(|side| side.name().to_owned()),
                change.fields().is_empty(),
            )
        })
    })?;

    assert_eq!(
        change,
        Some((
            RecordChangeKind::Changed,
            Some("Ship.Before".to_owned()),
            Some("Ship.After".to_owned()),
            true
        )),
        "the name differs, no field does"
    );
    Ok(())
}
