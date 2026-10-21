//! Reading the DataForge Entry of an Archive.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{Archive, DataForgeEntry, DataForgeLoadError, MAX_DATAFORGE_SIZE};
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
    Ok(Archive::open_with_key(
        path,
        Some(p4kx_testkit::test_key()),
    )?)
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
fn locating_finds_the_entry_read_without_reading_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let both = archive(
        dir.path(),
        vec![
            TestEntry::stored("Data\\Game.dcb", database(1)),
            TestEntry::stored("Data\\Game2.dcb", vec![0xFF; 4]),
        ],
    )?;
    let other = tempfile::tempdir()?;
    let none = archive(
        other.path(),
        vec![TestEntry::stored("Data\\Game3.dcb", database(1))],
    )?;

    let found = DataForgeEntry::locate(&both).map(p4kx_extraction::Entry::path);

    assert_eq!(found, Some("Data\\Game2.dcb"), "Game2.dcb, unread");
    assert!(DataForgeEntry::locate(&none).is_none(), "no DataForge");
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

#[test]
fn a_size_above_the_limit_is_rejected_before_reading() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::stored("Data\\Game2.dcb", database(1))
                .with_uncompressed_size(MAX_DATAFORGE_SIZE + 1),
        ],
    )?;

    let result = DataForgeEntry::read(&archive);

    assert!(
        matches!(
            result,
            Err(DataForgeLoadError::TooLarge { size, limit, .. })
                if size == MAX_DATAFORGE_SIZE + 1 && limit == MAX_DATAFORGE_SIZE
        ),
        "the forged size is rejected: {result:?}"
    );
    Ok(())
}

#[test]
fn a_forged_size_below_the_limit_fails_on_the_missing_data() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![TestEntry::stored("Data\\Game2.dcb", database(1)).with_uncompressed_size(1 << 30)],
    )?;

    let result = DataForgeEntry::read(&archive);

    assert!(
        matches!(result, Err(DataForgeLoadError::Read { .. })),
        "the Entry ends before its claimed size: {result:?}"
    );
    Ok(())
}
