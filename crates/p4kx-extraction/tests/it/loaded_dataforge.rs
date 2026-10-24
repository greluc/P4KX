// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A DataForge read once and kept open for many calls.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use p4kx_extraction::{Archive, DataForgeLoadError, LoadedDataForge};
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct};

type TestResult = Result<(), Box<dyn Error>>;

fn database(records: u128) -> Vec<u8> {
    (0..records)
        .fold(
            DataForgeBuilder::new().structure(TestStruct::new("Item")),
            |builder, guid| {
                builder.record(TestRecord::new(
                    guid + 1,
                    "Item",
                    format!("Item.N{guid}"),
                    format!("libs/foundry/records/item_{guid}.xml"),
                ))
            },
        )
        .build()
}

fn archive(dir: &Path, entries: Vec<TestEntry>) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        path,
        Some(p4kx_testkit::test_key()),
    )?)
}

#[test]
fn calls_share_the_open_dataforge_and_its_indexes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![TestEntry::zstd("Data\\Game2.dcb", database(5))],
    )?;
    let loaded = Arc::new(LoadedDataForge::load(&archive)?);
    assert_eq!(loaded.path(), "Data\\Game2.dcb", "the Entry read");
    let first = loaded.with(|forge| forge.path_order().as_ptr() as usize)?;
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let loaded = Arc::clone(&loaded);
            thread::spawn(move || {
                loaded.with(|forge| (forge.record_count(), forge.path_order().as_ptr() as usize))
            })
        })
        .collect();
    for worker in workers {
        let (count, order) = worker.join().map_err(|_| "a worker panicked")??;
        assert_eq!(count, 5, "every Record");
        assert_eq!(order, first, "the Record order is built once");
    }
    Ok(())
}

#[test]
fn an_archive_without_dataforge_does_not_load() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![TestEntry::stored("readme.txt", b"x".to_vec())],
    )?;
    assert!(
        matches!(
            LoadedDataForge::load(&archive),
            Err(DataForgeLoadError::Missing)
        ),
        "no DataForge Entry"
    );
    Ok(())
}

#[test]
fn a_corrupt_dataforge_does_not_load() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![TestEntry::stored("Data\\Game2.dcb", b"not a dcb".to_vec())],
    )?;
    assert!(
        matches!(
            LoadedDataForge::load(&archive),
            Err(DataForgeLoadError::Corrupt(_))
        ),
        "a corrupt database"
    );
    Ok(())
}
