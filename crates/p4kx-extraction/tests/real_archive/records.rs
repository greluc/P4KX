// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading the DataForge of a real Archive and building, writing and exporting every Record.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use p4kx_extraction::{
    Archive, DataForge, DataForgeEntry, DataForgeError, Record, RecordExportOptions, RecordXml,
    TreeBuilder, ValueError, export_records,
};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use super::{TestResult, report};

const KNOWN_RECORD: &str = "libs/foundry/records/entities/spaceships/aegs_avenger_titan";
const KNOWN_TYPE: &str = "EntityClassDefinition";
const KNOWN_GUID: &str = "0079c5d5-1678-4f8c-85ba-18ca8f642af6";
const DANGLING: &str = "dangling Reference";
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn reads_the_dataforge_of_a_real_archive() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;

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

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn builds_and_writes_every_record_of_a_real_archive() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let errors: Mutex<HashMap<String, usize>> = Mutex::default();
    let started = Instant::now();

    let failures: Vec<String> = forge
        .records()
        .collect::<Vec<_>>()
        .par_iter()
        .map_init(
            || (TreeBuilder::new(), RecordXml::new()),
            |(builder, xml), record| {
                let tree = builder.build(record).map_err(|error| error.to_string())?;
                let mut out = Vec::new();
                xml.write(&tree, &mut out)
                    .map_err(|error| error.to_string())?;
                let text = std::str::from_utf8(&out).map_err(|error| error.to_string())?;
                roxmltree::Document::parse(text).map_err(|error| format!("{error}: {text}"))?;
                if let Ok(mut errors) = errors.lock() {
                    for (_, error) in tree.errors() {
                        *errors.entry(error_kind(error)).or_default() += 1;
                    }
                }
                Ok::<_, String>(())
            },
        )
        .filter_map(Result::err)
        .collect();

    assert!(
        failures.is_empty(),
        "{} of {} Records fail, e.g. {:?}",
        failures.len(),
        forge.record_count(),
        failures.iter().take(10).collect::<Vec<_>>()
    );
    let errors = errors.into_inner().unwrap_or_default();
    report(&format!(
        "{} Records built and written, error values by kind {errors:?}, in {:?}",
        forge.record_count(),
        started.elapsed()
    ));
    let broken: usize = errors
        .iter()
        .filter(|(kind, _)| kind.as_str() != DANGLING)
        .map(|(_, count)| count)
        .sum();
    assert_eq!(broken, 0, "error values by kind: {errors:?}");
    Ok(())
}

fn error_kind(error: &ValueError) -> String {
    match error {
        ValueError::DanglingReference { .. } => DANGLING.to_owned(),
        other => other.to_string(),
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn exports_a_known_record_of_a_real_archive() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let records: Vec<_> = forge.records_at_path(KNOWN_RECORD).collect();
    let tree = TreeBuilder::new().build(records.first().ok_or("the known Record exists")?)?;
    assert_eq!(
        tree.errors().count(),
        0,
        "the known Record has no error value"
    );
    let target = tempfile::tempdir()?;

    let report = export_records(
        &forge,
        &records,
        target.path(),
        &RecordExportOptions::default(),
    );

    assert!(report.is_complete(), "{:?}", report.failures());
    let xml = std::fs::read_to_string(target.path().join(format!("{KNOWN_RECORD}.xml")))?;
    let document = roxmltree::Document::parse(&xml)?;
    let root = document.root_element();
    assert_eq!(root.tag_name().name(), KNOWN_TYPE, "type: {xml}");
    assert_eq!(root.attribute("guid"), Some(KNOWN_GUID), "GUID: {xml}");
    assert_eq!(
        root.attribute("path"),
        Some(KNOWN_RECORD),
        "record path: {xml}"
    );
    Ok(())
}
