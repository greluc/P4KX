//! Writes into a protected Star Citizen installation are refused by the services.

use std::error::Error;

use p4kx_extraction::{Archive, ExtractOptions, ExtractionError, extract, protect};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn extract_refuses_a_target_inside_the_installation_and_writes_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let installation = root.path().join("StarCitizen");
    let target = installation.join("LIVE").join("export");
    std::fs::create_dir_all(&target)?;
    let archive_path = root.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .write_to(&archive_path)?;
    let archive = Archive::open(&archive_path)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    protect(&installation);

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert_eq!(report.extracted(), 0, "nothing is extracted");
    let failure = report.failures().first().ok_or("one failure per Entry")?;
    assert!(
        matches!(failure.error(), ExtractionError::Installation(_)),
        "the failure is the typed refusal: {:?}",
        failure.error()
    );
    assert_eq!(
        std::fs::read_dir(&target)?.count(),
        0,
        "the folder stays empty"
    );

    let outside = root.path().join("out");
    std::fs::create_dir(&outside)?;
    let report = extract(&archive, &entries, &outside, &ExtractOptions::default());
    assert_eq!(report.extracted(), 1, "a folder outside is still written");
    Ok(())
}
