//! The Archive file is only ever opened for reading.

use std::error::Error;
use std::io::Read;

use p4kx_archive::Archive;
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn opens_and_reads_an_archive_that_nothing_may_write_to() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .entry(TestEntry::deflated(
            "Data\\b.xml",
            b"<b>second</b>".to_vec(),
        ))
        .write_to(&path)?;
    let before = std::fs::read(&path)?;
    let mut permissions = std::fs::metadata(&path)?.permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions)?;

    let archive = Archive::open(&path)?;
    let readers: Vec<_> = archive
        .entries()
        .iter()
        .map(|entry| archive.open_entry(entry))
        .collect::<Result<_, _>>()?;
    let mut data = Vec::new();
    for mut reader in readers {
        reader.read_to_end(&mut data)?;
    }
    drop(archive);

    assert_eq!(
        data, b"<a/><b>second</b>",
        "data read from a read-only file"
    );
    assert_eq!(std::fs::read(&path)?, before, "the file is unchanged");
    assert!(
        std::fs::metadata(&path)?.permissions().readonly(),
        "the file is still read-only"
    );
    Ok(())
}
