//! Extraction from a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::{HashMap, HashSet};
use std::error::Error;

use p4kx_extraction::{
    Archive, ConflictPolicy, DataForgeEntry, DataForgeError, Entry, ExtractOptions, extract,
};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const PER_KIND: usize = 10;
const MAX_SIZE: u64 = 4 << 20;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn extracts_a_small_subset_of_a_real_archive() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let subset = small_subset(archive.entries());
    let target = tempfile::tempdir()?;

    let report = extract(&archive, &subset, target.path(), &ExtractOptions::default());

    assert!(
        report.is_complete(),
        "every Entry is written: {:?}",
        report.failures()
    );
    assert_eq!(
        report.extracted(),
        subset.len(),
        "the whole subset is written"
    );
    for entry in &subset {
        let file = target.path().join(entry.path().replace('\\', "/"));
        assert_eq!(
            std::fs::metadata(&file)?.len(),
            entry.uncompressed_size(),
            "{} has its uncompressed size on disk",
            entry.path()
        );
    }

    let options = ExtractOptions {
        policy: ConflictPolicy::IfChanged,
        ..ExtractOptions::default()
    };
    let again = extract(&archive, &subset, target.path(), &options);

    assert_eq!(
        again.skipped().len(),
        subset.len(),
        "unchanged files are kept"
    );
    Ok(())
}

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
    match entry.open() {
        Ok(forge) => {
            assert!(forge.record_count() > 0, "the DataForge has Records");
            for record in forge.records() {
                record.name()?;
                record.type_name()?;
                let path = record.record_path()?;
                assert!(
                    forge
                        .records_at_path(path)
                        .any(|found| found.guid() == record.guid()),
                    "{path} finds its Record"
                );
                assert!(
                    forge.record_by_guid(record.guid()).is_some(),
                    "{} finds its Record",
                    record.guid()
                );
            }
        }
        Err(DataForgeError::UnsupportedVersion { version }) => {
            assert!(version > 6, "only a newer version is rejected: {version}");
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn small_subset(entries: &[Entry]) -> Vec<&Entry> {
    let mut per_kind = HashMap::new();
    let mut seen = HashSet::new();
    entries
        .iter()
        .filter(|entry| entry.uncompressed_size() <= MAX_SIZE)
        .filter(|entry| seen.insert(entry.path().replace('/', "\\").to_lowercase()))
        .filter(|entry| {
            let count = per_kind
                .entry((entry.compression(), entry.is_encrypted()))
                .or_insert(0);
            *count += 1;
            *count <= PER_KIND
        })
        .collect()
}
