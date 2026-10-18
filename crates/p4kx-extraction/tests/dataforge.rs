//! Reading the DataForge Entry of an Archive.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{Archive, DataForgeEntry, DataForgeLoadError};
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
                    "libs/foundry/records/items.xml",
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
    Ok(Archive::open(path)?)
}

#[test]
fn prefers_game2_over_game() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::stored("Data\\Game.dcb", database(1)),
            TestEntry::zstd("Data\\Game2.dcb", database(2)).encrypted(),
        ],
    )?;

    let entry = DataForgeEntry::read(&archive)?;

    assert_eq!(entry.path(), "Data\\Game2.dcb", "Game2.dcb is preferred");
    assert_eq!(entry.open()?.record_count(), 2, "its Records are read");
    Ok(())
}

#[test]
fn falls_back_to_game_dcb() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::stored("Data\\readme.txt", b"hello".to_vec()),
            TestEntry::deflated("data/GAME.DCB", database(3)),
        ],
    )?;

    let entry = DataForgeEntry::read(&archive)?;

    assert_eq!(
        entry.path(),
        "data/GAME.DCB",
        "case and separator do not matter"
    );
    assert_eq!(entry.open()?.record_count(), 3, "its Records are read");
    Ok(())
}

#[test]
fn an_archive_without_dataforge_is_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![TestEntry::stored("Data\\Game3.dcb", database(1))],
    )?;

    let result = DataForgeEntry::read(&archive);

    assert!(
        matches!(result, Err(DataForgeLoadError::Missing)),
        "no DataForge Entry: {result:?}"
    );
    Ok(())
}

#[test]
fn corrupt_entry_data_is_a_read_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![TestEntry::deflated("Data\\Game2.dcb", database(1)).with_payload(vec![0xFF; 8])],
    )?;

    let result = DataForgeEntry::read(&archive);

    assert!(
        matches!(result, Err(DataForgeLoadError::Read { .. })),
        "the Entry cannot be decompressed: {result:?}"
    );
    Ok(())
}
