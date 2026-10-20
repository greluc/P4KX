//! Verifying a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;

use p4kx_extraction::{Archive, Entry, VerifyOptions, verify};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const STEP: usize = 997;
const MAX_SIZE: u64 = 64 << 20;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_a_real_archive_verifies_without_a_mismatch() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let sample: Vec<&Entry> = archive
        .entries()
        .iter()
        .step_by(STEP)
        .filter(|entry| entry.uncompressed_size() <= MAX_SIZE)
        .collect();

    let report = verify(&archive, &sample, &VerifyOptions::default());

    assert!(!sample.is_empty(), "the sample holds Entries");
    assert_eq!(report.checksum_mismatches().count(), 0, "no mismatch");
    assert!(
        report.is_complete(),
        "every sampled Entry verifies: {:?}",
        report.failures()
    );
    assert_eq!(report.verified(), sample.len(), "all were checked");
    Ok(())
}
