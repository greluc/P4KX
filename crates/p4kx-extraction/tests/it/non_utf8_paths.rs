// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Extracting Entries whose stored paths are not UTF-8.

use std::error::Error;

use p4kx_extraction::{Archive, ExtractOptions, extract};
use p4kx_testkit::{ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn entries_with_equal_lossy_paths_extract_to_distinct_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("x", b"first".to_vec()).with_path_bytes(b"Data\\a\xE9.txt"))
        .entry(TestEntry::stored("x", b"second".to_vec()).with_path_bytes(b"Data\\a\xE8.txt"))
        .entry(TestEntry::stored("Data\\plain.txt", b"third".to_vec()))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let target = dir.path().join("out");
    std::fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert_eq!(report.extracted(), 3, "no Entry overwrites another");
    let data = target.join("Data");
    assert_eq!(std::fs::read(data.join("a%E9.txt"))?, b"first");
    assert_eq!(std::fs::read(data.join("a%E8.txt"))?, b"second");
    assert_eq!(std::fs::read(data.join("plain.txt"))?, b"third");
    Ok(())
}

#[test]
fn a_converting_extraction_keeps_equal_lossy_paths_apart() -> TestResult {
    let cryxml = |name: &str| {
        TestElement::new("Ship")
            .attribute("name", name)
            .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
    };
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("x", cryxml("first")).with_path_bytes(b"Data\\a\xE9.xml"))
        .entry(TestEntry::zstd("x", cryxml("second")).with_path_bytes(b"Data\\a\xE8.xml"))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let target = dir.path().join("out");
    std::fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let options = ExtractOptions {
        convert: true,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &entries, &target, &options);

    assert_eq!(report.extracted(), 2, "no Entry overwrites another");
    let data = target.join("Data");
    assert!(String::from_utf8(std::fs::read(data.join("a%E9.xml"))?)?.contains("first"));
    assert!(String::from_utf8(std::fs::read(data.join("a%E8.xml"))?)?.contains("second"));
    Ok(())
}
