// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Checking Entry checksums during an Extraction and in a verification run.

use std::error::Error;
use std::fs;

use p4kx_extraction::{
    Archive, Checksum, ConflictPolicy, Entry, ExtractOptions, ExtractionError, VerifyOptions,
    extract, verify,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry, test_key};

type TestResult = Result<(), Box<dyn Error>>;

const DATA: &[u8] = b"verified payload for the extraction tests";

fn open(builder: &ArchiveBuilder) -> Result<(tempfile::TempDir, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(test_key()))?;
    Ok((dir, archive))
}

fn mixed() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\good.bin", DATA))
        .entry(TestEntry::deflated("Data\\bad_deflate.bin", DATA).with_crc32c(1))
        .entry(TestEntry::zstd("Data\\bad_zstd.bin", DATA).with_crc32c(2))
        .entry(
            TestEntry::stored("Data\\bad_encrypted.bin", DATA)
                .encrypted()
                .with_crc32c(3),
        )
        .entry(TestEntry::zstd("Data\\also_good.bin", DATA).encrypted())
}

#[test]
fn verification_reports_every_mismatch_and_counts_the_good_entries() -> TestResult {
    let (_dir, archive) = open(&mixed())?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();

    let report = verify(&archive, &entries, &VerifyOptions::default());

    assert_eq!(report.verified(), 2, "the two correct Entries pass");
    let bad: Vec<_> = report
        .checksum_mismatches()
        .map(|failure| {
            (
                failure.path().to_owned(),
                failure.checksum_mismatch().map(|(expected, _)| expected),
            )
        })
        .collect();
    assert_eq!(
        bad,
        [
            (
                "Data\\bad_deflate.bin".to_owned(),
                Some(Checksum::Crc32c(1))
            ),
            ("Data\\bad_zstd.bin".to_owned(), Some(Checksum::Crc32c(2))),
            (
                "Data\\bad_encrypted.bin".to_owned(),
                Some(Checksum::Crc32c(3))
            ),
        ],
        "the mismatches in Entry order with their stored checksum"
    );
    assert!(!report.is_complete(), "mismatches make the run incomplete");
    Ok(())
}

#[test]
fn verification_reports_unreadable_data_apart_from_a_mismatch() -> TestResult {
    let truncated = TestEntry::deflated("Data\\cut.bin", DATA);
    let payload = truncated.payload().get(..4).unwrap_or_default().to_vec();
    let (_dir, archive) = open(&ArchiveBuilder::new().entry(truncated.with_payload(payload)))?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();

    let report = verify(&archive, &entries, &VerifyOptions::default());

    assert_eq!(report.failures().len(), 1, "the Entry fails");
    assert_eq!(
        report.checksum_mismatches().count(),
        0,
        "undecodable data is no mismatch"
    );
    assert!(
        matches!(
            report
                .failures()
                .first()
                .map(p4kx_extraction::EntryFailure::error),
            Some(ExtractionError::Read(_))
        ),
        "it is a read failure"
    );
    Ok(())
}

#[test]
fn extraction_without_verify_writes_a_mismatching_entry() -> TestResult {
    let (dir, archive) = open(&mixed())?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();
    let target = dir.path().join("out");
    fs::create_dir(&target)?;

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert_eq!(report.extracted(), 5, "nothing is checked by default");
    assert!(report.failures().is_empty(), "no failure");
    Ok(())
}

#[test]
fn extraction_with_verify_fails_the_mismatching_entries_and_writes_no_file_for_them() -> TestResult
{
    let (dir, archive) = open(&mixed())?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();
    let target = dir.path().join("out");
    fs::create_dir(&target)?;
    let options = ExtractOptions {
        verify: true,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &entries, &target, &options);

    assert_eq!(report.extracted(), 2, "the correct Entries are written");
    assert_eq!(report.checksum_mismatches().count(), 3, "three mismatches");
    for name in ["bad_deflate.bin", "bad_zstd.bin", "bad_encrypted.bin"] {
        assert!(
            !target.join("Data").join(name).exists(),
            "{name} is not put in place"
        );
    }
    assert_eq!(
        fs::read(target.join("Data").join("good.bin"))?,
        DATA,
        "a correct Entry is written"
    );
    assert!(!report.is_complete(), "mismatches fail the run");
    Ok(())
}

#[test]
fn extraction_with_verify_also_checks_converted_cryxml() -> TestResult {
    let xml = p4kx_testkit::TestElement::new("Root").to_cryxml(
        p4kx_testkit::CryXmlSignature::CryXmlB,
        p4kx_testkit::CryXmlByteOrder::Little,
    );
    let (dir, archive) =
        open(&ArchiveBuilder::new().entry(TestEntry::stored("Data\\a.xml", xml).with_crc32c(9)))?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();
    let target = dir.path().join("out");
    fs::create_dir(&target)?;
    let options = ExtractOptions {
        verify: true,
        convert: true,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &entries, &target, &options);

    assert_eq!(
        report.checksum_mismatches().count(),
        1,
        "the CryXml Entry mismatches"
    );
    assert!(
        !target.join("Data").join("a.xml").exists(),
        "nothing is written"
    );
    Ok(())
}

#[test]
fn a_mismatch_keeps_an_existing_file_under_overwrite_and_leaves_no_temporary_file() -> TestResult {
    let (dir, archive) =
        open(&ArchiveBuilder::new().entry(TestEntry::zstd("Data\\a.bin", DATA).with_crc32c(5)))?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();
    let target = dir.path().join("out");
    fs::create_dir_all(target.join("Data"))?;
    fs::write(target.join("Data").join("a.bin"), b"old content")?;
    let options = ExtractOptions {
        policy: ConflictPolicy::Overwrite,
        verify: true,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &entries, &target, &options);

    assert_eq!(report.checksum_mismatches().count(), 1, "one mismatch");
    assert_eq!(
        fs::read(target.join("Data").join("a.bin"))?,
        b"old content",
        "the existing file survives"
    );
    let names: Vec<_> = fs::read_dir(target.join("Data"))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<_, _>>()?;
    assert_eq!(names.len(), 1, "no temporary file is left: {names:?}");
    Ok(())
}

#[test]
fn verification_checks_the_sha_256_of_the_stored_bytes_too() -> TestResult {
    let wrong = [0x5A; 32];
    let builder = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\good.bin", DATA).with_sha256())
        .entry(TestEntry::zstd("Data\\bad.bin", DATA).with_sha256_value(wrong))
        .entry(
            TestEntry::zstd("Data\\secret.bin", DATA)
                .encrypted()
                .with_sha256(),
        );
    let (dir, archive) = open(&builder)?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();

    let report = verify(&archive, &entries, &VerifyOptions::default());

    assert_eq!(report.verified(), 2, "the correct digests pass");
    let bad: Vec<_> = report
        .checksum_mismatches()
        .map(|failure| (failure.path().to_owned(), failure.checksum_mismatch()))
        .collect();
    assert_eq!(bad.len(), 1, "one mismatch");
    let (path, mismatch) = bad.into_iter().next().ok_or("no mismatch")?;
    assert_eq!(path, "Data\\bad.bin", "the Entry with the wrong digest");
    assert_eq!(
        mismatch.map(|(expected, _)| expected),
        Some(Checksum::Sha256(wrong)),
        "the stated digest"
    );

    let target = dir.path().join("out");
    fs::create_dir(&target)?;
    let options = ExtractOptions {
        verify: true,
        ..ExtractOptions::default()
    };
    let report = extract(&archive, &entries, &target, &options);
    assert_eq!(
        report.extracted(),
        2,
        "the Entry with the wrong digest is not written"
    );
    assert!(
        !target.join("Data").join("bad.bin").exists(),
        "no file for it"
    );
    Ok(())
}
