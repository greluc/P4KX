// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The M2 verification over a real Archive: plausible DataForge counts, the open unp4k cases and
//! the Record files against the Entries of the Archive.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use p4kx_extraction::{Archive, DataForgeEntry, Entry};
use p4kx_testkit::real_archive_path;

use super::{TestResult, report};

const MIN_RECORDS: usize = 20_000;
const MIN_STRUCTS: usize = 1_000;
const MIN_ENUMS: usize = 100;

fn normalised(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_dataforge_of_a_real_archive_has_plausible_counts() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let started = Instant::now();

    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;

    report(&format!(
        "{}: version {}, {} Records, {} structs, {} properties, {} enums, {} data mappings, \
         read and opened in {:?}",
        entry.path(),
        forge.version(),
        forge.record_count(),
        forge.struct_count(),
        forge.property_count(),
        forge.enum_count(),
        forge.data_mapping_count(),
        started.elapsed()
    ));
    assert!(
        matches!(forge.version(), 6 | 8),
        "version {}",
        forge.version()
    );
    assert!(forge.record_count() >= MIN_RECORDS, "Records");
    assert!(forge.struct_count() >= MIN_STRUCTS, "structs");
    assert!(forge.enum_count() >= MIN_ENUMS, "enums");
    assert!(forge.property_count() >= forge.struct_count(), "properties");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_open_unp4k_cases_do_not_occur_in_a_real_archive() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;

    assert!(
        forge.data_mapping_count() > 0,
        "opening checked that the data mappings address the instance data exactly"
    );
    let mut guids = HashSet::new();
    for record in forge.records() {
        let found = forge
            .record_by_guid(record.guid())
            .ok_or_else(|| format!("{} is not found by its GUID", record.guid()))?;
        assert_eq!(found.guid(), record.guid(), "the GUID finds its Record");
        record
            .record_path()
            .map_err(|error| format!("{}: {error}", record.guid()))?;
        assert!(guids.insert(record.guid()), "{} is unique", record.guid());
    }
    assert_eq!(guids.len(), forge.record_count(), "every Record has a root");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn no_record_file_replaces_an_entry_of_a_real_archive() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let folder = normalised(entry.path())
        .rsplit_once('/')
        .map_or(String::new(), |(folder, _)| format!("{folder}/"));
    let mut per_path: HashMap<String, usize> = HashMap::new();
    for record in forge.records() {
        if let Ok(path) = record.record_path() {
            *per_path.entry(path.to_lowercase()).or_default() += 1;
        }
    }
    let (alone, shared): (Vec<_>, Vec<_>) =
        per_path.into_iter().partition(|(_, count)| *count == 1);
    let below = |records: &[(String, usize)], end: &str| -> HashSet<String> {
        records
            .iter()
            .map(|(path, _)| format!("{folder}{path}{end}"))
            .collect()
    };
    let (files, folders) = (below(&alone, ".xml"), below(&shared, ""));
    let records_folder = format!("{folder}libs/foundry/records/");

    let under: Vec<&str> = archive
        .entries()
        .iter()
        .map(Entry::path)
        .filter(|path| normalised(path).starts_with(&records_folder))
        .collect();
    let replaced: Vec<&str> = archive
        .entries()
        .iter()
        .map(Entry::path)
        .filter(|path| {
            let path = normalised(path);
            files.contains(&path)
                || folders.contains(&path)
                || path.match_indices('/').any(|(at, _)| {
                    path.get(..at)
                        .is_some_and(|parent| folders.contains(parent))
                })
        })
        .collect();

    report(&format!(
        "{} Records in {} single files and {} shared folders below {folder}; {} Entries lie under \
         {records_folder} ({under:?}), {} of all Entries would be replaced",
        forge.record_count(),
        files.len(),
        folders.len(),
        under.len(),
        replaced.len()
    ));
    assert!(
        replaced.is_empty(),
        "Entries replaced by Record files, e.g. {:?}",
        replaced.iter().take(10).collect::<Vec<_>>()
    );
    Ok(())
}
