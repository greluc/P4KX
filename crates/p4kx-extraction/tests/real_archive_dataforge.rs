//! Reading the DataForge of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;

use p4kx_extraction::{Archive, DataForge, DataForgeEntry, DataForgeError, Record};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn reads_the_dataforge_of_a_real_archive() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;

    let entry = DataForgeEntry::read(&archive)?;

    assert!(
        entry.path().eq_ignore_ascii_case("Data\\Game2.dcb"),
        "Game2.dcb is preferred: {}",
        entry.path()
    );
    let forge = entry.open()?;
    assert!(
        matches!(forge.version(), 6 | 8),
        "a supported version: {}",
        forge.version()
    );
    assert!(forge.record_count() > 0, "the DataForge has Records");
    let failures: Vec<String> = forge
        .records()
        .par_bridge()
        .filter_map(|record| check_record(&forge, record).err())
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} Records fail, e.g. {:?}",
        failures.len(),
        forge.record_count(),
        failures.iter().take(10).collect::<Vec<_>>()
    );
    Ok(())
}

fn check_record(forge: &DataForge<'_>, record: Record<'_>) -> Result<(), String> {
    let failed = |error: DataForgeError| format!("{}: {error}", record.guid());
    record.name().map_err(failed)?;
    record.type_name().map_err(failed)?;
    let tag = record.tag().map_err(failed)?;
    if tag.is_some() != (forge.version() == 8) {
        return Err(format!("{}: tag {tag:?}", record.guid()));
    }
    let path = record.record_path().map_err(failed)?;
    if !forge
        .records_at_path(path)
        .any(|found| found.guid() == record.guid())
    {
        return Err(format!("{path} does not find its Record"));
    }
    if forge.record_by_guid(record.guid()).is_none() {
        return Err(format!("{} does not find its Record", record.guid()));
    }
    Ok(())
}
