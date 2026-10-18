//! An owned Comparison on the Record level, compared again for its report.

mod forge_pair;

use std::error::Error;
use std::sync::Arc;

use forge_pair::ForgePair;
use p4kx_comparison::{
    Archive, CompareOptions, ComparisonError, DataForgeEntry, Levels, OwnedComparison,
    OwnedDataForges,
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
    let old = Archive::open(pair.old.path())?;
    let new = Archive::open(pair.new.path())?;
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
