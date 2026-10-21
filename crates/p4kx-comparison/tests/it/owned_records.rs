//! An owned Comparison on the Record level, compared again for its report.

use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::forge_pair::ForgePair;
use p4kx_comparison::{
    Archive, CancelToken, CompareOptions, ComparisonError, ComparisonProgress, DataForgeEntry,
    Filter, Levels, OwnedComparison, OwnedDataForges, RecompareError, RecordChange,
    RecordChangeKind, RecordKey,
};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

fn database(speed: i32) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Ship").field("speed", TestType::Int32))
        .record(
            TestRecord::new(1, "Ship", "Ship.Tuned", "libs/ships/tuned.xml")
                .value("speed", TestValue::Int32(speed)),
        )
}

fn owned(
    pair: &ForgePair,
    levels: Levels,
    forges: bool,
) -> Result<OwnedComparison, Box<dyn Error>> {
    let old = Archive::open_with_key(pair.old.path(), Some(p4kx_testkit::test_key()))?;
    let new = Archive::open_with_key(pair.new.path(), Some(p4kx_testkit::test_key()))?;
    let dataforges = forges
        .then(|| -> Result<_, Box<dyn Error>> {
            Ok(OwnedDataForges {
                old: Arc::new(DataForgeEntry::read(&old)?),
                new: Arc::new(DataForgeEntry::read(&new)?),
            })
        })
        .transpose()?;
    let options = CompareOptions {
        levels,
        ..CompareOptions::default()
    };
    Ok(OwnedComparison::compare(
        Arc::new(old),
        Arc::new(new),
        dataforges,
        &options,
    )?)
}

#[test]
fn the_recomparison_holds_the_record_level_like_compare() -> TestResult {
    let pair = ForgePair::new(&database(4), &database(40))?;
    let options = CompareOptions {
        levels: Levels::ALL,
        ..CompareOptions::default()
    };
    let expected = pair.compare_with(&options, |comparison| serde_json::to_value(comparison))??;

    let owned = owned(&pair, Levels::ALL, true)?;
    let report = owned.recompare(None, None, |comparison| serde_json::to_value(comparison))??;

    assert_eq!(report, expected, "the same report, Records included");
    assert_eq!(
        pair.compare_records(|comparison| comparison.record_counts().changed)?,
        1,
        "the Record level alone sees the change too"
    );
    assert_eq!(
        owned.recompare(None, None, |comparison| comparison.record_counts().changed)?,
        1,
        "the tuned Record changed"
    );
    Ok(())
}

fn twins(paths: &[&str]) -> DataForgeBuilder {
    paths.iter().zip(1..).fold(
        DataForgeBuilder::new().structure(TestStruct::new("Ship").field("speed", TestType::Int32)),
        |builder, (path, speed)| {
            builder.record(
                TestRecord::new(7, "Ship", "Ship.Twin", *path)
                    .value("speed", TestValue::Int32(speed)),
            )
        },
    )
}

type Summary = (RecordChangeKind, String, Vec<String>);

fn summary(change: &RecordChange<'_>) -> Summary {
    (
        change.kind(),
        change.path().to_owned(),
        change
            .fields()
            .iter()
            .map(|field| field.path().as_str().to_owned())
            .collect(),
    )
}

#[test]
fn the_record_changes_are_those_of_compare_and_report_their_progress() -> TestResult {
    let pair = ForgePair::new(&database(4), &database(40))?;
    let expected = pair.compare_records(|comparison| {
        comparison
            .record_changes()
            .iter()
            .map(summary)
            .collect::<Vec<_>>()
    })?;
    let owned = owned(&pair, Levels::ALL, true)?;
    let seen = Mutex::new(Vec::new());
    let progress = |progress: ComparisonProgress| {
        if let Ok(mut seen) = seen.lock() {
            seen.push((progress.records_done, progress.records_total));
        }
    };

    let changes = owned.record_changes(None, None, Some(&progress), |changes| {
        changes.iter().map(summary).collect::<Vec<_>>()
    })?;

    assert_eq!(changes, expected, "the same Record changes");
    let seen = seen.into_inner()?;
    assert_eq!(seen.last(), Some(&(1, 1)), "the end is reported: {seen:?}");
    Ok(())
}

#[test]
fn the_record_changes_narrow_to_the_filter() -> TestResult {
    let pair = ForgePair::new(&database(4), &database(40))?;
    let owned = owned(&pair, Levels::RECORDS, true)?;

    let count = owned.record_changes(
        Some(&Filter::new("libs/weapons/*")),
        None,
        None,
        |changes| changes.len(),
    )?;

    assert_eq!(count, 0, "no Record below that path");
    Ok(())
}

#[test]
fn one_record_change_is_compared_again_by_its_key() -> TestResult {
    let pair = ForgePair::new(
        &twins(&["libs/ships/first.xml", "libs/ships/second.xml"]),
        &twins(&["libs/ships/first.xml", "libs/ships/moved.xml"]),
    )?;
    let owned = owned(&pair, Levels::RECORDS, true)?;
    let listed = owned.record_changes(None, None, None, |changes| {
        changes
            .iter()
            .map(|change| (change.key(), summary(change)))
            .collect::<Vec<_>>()
    })?;

    let mut again = Vec::new();
    for (key, _) in &listed {
        again.push((
            *key,
            owned.with_records(|lookup| lookup.change(*key).as_ref().map(summary))?,
        ));
    }

    assert_eq!(listed.len(), 1, "only the second twin moved: {listed:?}");
    for ((key, expected), (_, found)) in listed.iter().zip(&again) {
        assert_eq!(
            found.as_ref(),
            Some(expected),
            "the same change for {key:?}"
        );
    }
    assert_eq!(
        listed
            .first()
            .map(|(key, _)| (key.old_position(), key.new_position())),
        Some((Some(1), Some(1))),
        "the positions of the second twin on both sides"
    );
    Ok(())
}

#[test]
fn a_key_outside_both_dataforges_has_no_change() -> TestResult {
    let twins_pair = ForgePair::new(
        &twins(&["libs/ships/first.xml", "libs/ships/second.xml"]),
        &twins(&["libs/ships/first.xml", "libs/ships/moved.xml"]),
    )?;
    let foreign =
        owned(&twins_pair, Levels::RECORDS, true)?.record_changes(None, None, None, |changes| {
            changes.first().map(RecordChange::key)
        })?;
    let pair = ForgePair::new(&database(4), &database(40))?;
    let owned = owned(&pair, Levels::RECORDS, true)?;

    let (missing, neither) = owned.with_records(|lookup| {
        (
            foreign.and_then(|key| lookup.change(key)).is_some(),
            lookup.change(RecordKey::default()).is_some(),
        )
    })?;

    assert!(foreign.is_some(), "a key of the other pair");
    assert!(!missing, "no Record at the second position");
    assert!(!neither, "no side at all");
    Ok(())
}

#[test]
fn comparing_the_records_stops_once_cancelled() -> TestResult {
    let pair = ForgePair::new(&database(4), &database(40))?;
    let owned = owned(&pair, Levels::RECORDS, true)?;
    let cancel = CancelToken::new();
    cancel.cancel();

    let result = owned.record_changes(None, Some(&cancel), None, |changes| changes.len());

    assert!(
        matches!(
            result,
            Err(RecompareError::Comparison(ComparisonError::Cancelled))
        ),
        "cancelled: {result:?}"
    );
    Ok(())
}

#[test]
fn the_record_level_needs_the_dataforge_entries() -> TestResult {
    let pair = ForgePair::new(&database(4), &database(40))?;

    let missing = owned(&pair, Levels::RECORDS, false);

    let error = missing.err().map(|error| error.to_string());
    assert_eq!(
        error,
        Some(ComparisonError::NoDataForge.to_string()),
        "no DataForge Entries"
    );
    Ok(())
}
