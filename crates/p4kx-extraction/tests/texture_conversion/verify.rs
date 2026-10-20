use std::error::Error;

use p4kx_extraction::{ConflictPolicy, ExtractOptions, extract};

use crate::fixture::{Fixture, HEADER, files, rock, texture_entries};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_part_with_a_wrong_checksum_fails_a_verified_texture_run() -> TestResult {
    let mut entries = texture_entries(&rock());
    let part = entries.remove(2).with_crc32c(1);
    entries.insert(2, part);
    let fixture = Fixture::new(entries)?;
    let all: Vec<_> = fixture.archive.entries().iter().collect();
    let options = ExtractOptions {
        policy: ConflictPolicy::Skip,
        convert: true,
        verify: true,
        ..ExtractOptions::default()
    };

    let report = extract(&fixture.archive, &all, &fixture.target, &options);

    let bad: Vec<_> = report
        .checksum_mismatches()
        .map(p4kx_extraction::EntryFailure::path)
        .collect();
    assert_eq!(
        bad,
        [format!("{HEADER}.2")],
        "the part is reported as a mismatch"
    );
    assert!(!report.is_complete(), "the wrong checksum fails the run");
    assert_eq!(
        files(&fixture.target)?,
        [
            "Data/Textures/rock_diff.dds",
            "Data/Textures/rock_diff.dds.1",
            "Data/Textures/rock_diff.dds.3"
        ],
        "the mismatching part is not written and the texture falls back to its raw Entries"
    );
    Ok(())
}
