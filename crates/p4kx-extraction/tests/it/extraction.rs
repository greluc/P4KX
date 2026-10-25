// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Running an Extraction through the public API.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{
    Archive, ExtractOptions, ExtractionError, UnknownEntries, extract, select_entries,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn open(dir: &Path, builder: &ArchiveBuilder) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

fn sample() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\a.xml", b"<a/>".to_vec()))
        .entry(TestEntry::stored("Data\\b.txt", b"bee".to_vec()))
        .entry(TestEntry::zstd("Data\\secret.xml", b"<s/>".to_vec()).encrypted())
        .entry(
            TestEntry::deflated("Data\\broken.xml", b"<b/>".to_vec()).with_payload(vec![0xFF; 8]),
        )
}

#[test]
fn selects_requested_entries_by_path_with_either_separator_and_any_case() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path(), &sample())?;

    let selected = select_entries(&archive, &["data/b.TXT", "Data\\Libs\\a.xml"])?;

    let paths: Vec<_> = selected.iter().map(|entry| entry.path()).collect();
    assert_eq!(
        paths,
        vec!["Data\\Libs\\a.xml", "Data\\b.txt"],
        "both Entries are selected in Archive order"
    );
    Ok(())
}

#[test]
fn selecting_an_unknown_path_names_every_unknown_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path(), &sample())?;

    let result = select_entries(&archive, &["Data\\b.txt", "Data\\nope.xml", "x"]);

    let Err(UnknownEntries(unknown)) = result else {
        return Err(format!("unknown paths are an error: {result:?}").into());
    };
    assert_eq!(unknown, vec!["Data\\nope.xml", "x"], "unknown paths");
    Ok(())
}

#[test]
fn an_unknown_path_requested_twice_is_reported_twice() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path(), &sample())?;

    let result = select_entries(&archive, &["x", "Data\\b.txt", "x"]);

    let Err(UnknownEntries(unknown)) = result else {
        return Err(format!("unknown paths are an error: {result:?}").into());
    };
    assert_eq!(unknown, vec!["x", "x"], "once per request");
    Ok(())
}

#[test]
fn selecting_many_paths_keeps_archive_order_and_finds_entries_that_differ_only_in_case()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let mut builder = ArchiveBuilder::new();
    for index in 0..300 {
        builder = builder.entry(TestEntry::stored(
            format!("Data\\Part{index}\\file.bin"),
            vec![1; 2],
        ));
    }
    let builder = builder
        .entry(TestEntry::stored("Data\\Twin.txt", b"1".to_vec()))
        .entry(TestEntry::stored("DATA\\TWIN.TXT", b"2".to_vec()));
    let archive = open(dir.path(), &builder)?;

    let selected = select_entries(
        &archive,
        &[
            "data/twin.txt",
            "DATA/PART250/FILE.BIN",
            "data/part7/file.bin",
            "data/twin.txt",
        ],
    )?;

    let paths: Vec<_> = selected.iter().map(|entry| entry.path()).collect();
    assert_eq!(
        paths,
        vec![
            "Data\\Part7\\file.bin",
            "Data\\Part250\\file.bin",
            "Data\\Twin.txt",
            "DATA\\TWIN.TXT"
        ],
        "every Entry at a requested path, once, in Archive order"
    );
    let Err(UnknownEntries(unknown)) = select_entries(&archive, &["data/part300/file.bin"]) else {
        return Err("a path past the last Entry is unknown".into());
    };
    assert_eq!(unknown, vec!["data/part300/file.bin"], "unknown path");
    Ok(())
}

#[test]
fn a_run_limited_to_some_threads_writes_the_same_files_and_report() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut builder = ArchiveBuilder::new();
    for index in 0..40 {
        builder = builder.entry(TestEntry::stored(
            format!("Data\\Dir{}\\file{index}.txt", index % 4),
            format!("content {index}").into_bytes(),
        ));
    }
    let archive = open(dir.path(), &builder)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    for threads in [1, 3] {
        let target = dir.path().join(format!("out{threads}"));
        std::fs::create_dir(&target)?;
        let options = ExtractOptions {
            threads: std::num::NonZeroUsize::new(threads),
            ..ExtractOptions::default()
        };

        let report = extract(&archive, &entries, &target, &options);

        assert_eq!(report.extracted(), 40, "{threads} threads extract all");
        assert!(report.is_complete(), "{threads} threads: {report:?}");
        assert_eq!(
            std::fs::read(target.join("Data").join("Dir3").join("file39.txt"))?,
            b"content 39",
            "content with {threads} threads"
        );
    }
    Ok(())
}

#[test]
fn a_link_in_the_place_of_a_folder_is_refused_for_every_entry_of_that_folder() -> TestResult {
    let dir = tempfile::tempdir()?;
    let builder = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Dir\\one.txt", b"1".to_vec()))
        .entry(TestEntry::stored("Data\\Dir\\two.txt", b"2".to_vec()));
    let archive = open(dir.path(), &builder)?;
    let target = dir.path().join("out");
    std::fs::create_dir(&target)?;
    std::fs::write(target.join("Data"), b"a file where the folder belongs")?;
    let entries: Vec<_> = archive.entries().iter().collect();

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert_eq!(report.extracted(), 0, "nothing is written");
    assert_eq!(report.failures().len(), 2, "both Entries fail");
    Ok(())
}

#[test]
fn extracts_entries_and_reports_counts_and_failures() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path(), &sample())?;
    let target = dir.path().join("out");
    std::fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert_eq!(report.extracted(), 3, "three Entries are extracted");
    assert_eq!(
        std::fs::read(target.join("Data").join("Libs").join("a.xml"))?,
        b"<a/>",
        "nested Entry content"
    );
    assert_eq!(
        std::fs::read(target.join("Data").join("b.txt"))?,
        b"bee",
        "Entry content"
    );
    assert_eq!(
        std::fs::read(target.join("Data").join("secret.xml"))?,
        b"<s/>",
        "encrypted Entry content"
    );
    let failures: Vec<_> = report
        .failures()
        .iter()
        .map(p4kx_extraction::EntryFailure::path)
        .collect();
    assert_eq!(
        failures,
        vec!["Data\\broken.xml"],
        "the corrupt Entry fails"
    );
    assert!(
        !target.join("Data").join("broken.xml").exists(),
        "no file for the failed Entry"
    );
    assert!(!report.is_complete(), "a failure makes the run incomplete");
    Ok(())
}

#[test]
fn existing_files_are_skipped_and_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path(), &sample())?;
    let target = dir.path().join("out");
    std::fs::create_dir_all(target.join("Data"))?;
    std::fs::write(target.join("Data").join("b.txt"), "mine")?;
    let entries = select_entries(&archive, &["Data\\b.txt"])?;

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert_eq!(
        report.skipped(),
        ["Data\\b.txt"],
        "the existing file is skipped"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("Data").join("b.txt"))?,
        "mine",
        "the existing file is unchanged"
    );
    assert!(report.is_complete(), "skipping is not a failure");
    Ok(())
}

#[test]
fn entry_paths_that_leave_the_target_are_rejected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let unsafe_paths = [
        "..\\..\\escaped.txt",
        "Data\\..\\..\\escaped.txt",
        "\\rooted.txt",
        "/rooted.txt",
        "C:\\drive.txt",
        "C:drive.txt",
        "\\\\server\\share\\unc.txt",
        "Data\\.. \\escaped.txt",
        "Data\\",
        "",
    ];
    let builder = unsafe_paths
        .iter()
        .fold(ArchiveBuilder::new(), |builder, path| {
            builder.entry(TestEntry::stored(*path, b"evil".to_vec()))
        });
    let archive = open(dir.path(), &builder)?;
    let target = dir.path().join("a").join("b").join("out");
    std::fs::create_dir_all(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert_eq!(report.extracted(), 0, "nothing is extracted");
    assert_eq!(
        report.failures().len(),
        unsafe_paths.len(),
        "every unsafe path fails"
    );
    assert!(
        report
            .failures()
            .iter()
            .all(|failure| matches!(failure.error(), ExtractionError::UnsafePath)),
        "every failure is an unsafe path: {:?}",
        report.failures()
    );
    assert_eq!(
        count_files(dir.path())?,
        1,
        "no file besides the Archive is written anywhere"
    );
    Ok(())
}

#[test]
fn extracts_every_compression_and_encryption_with_the_original_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let data: Vec<u8> = (0..200_000_u32).flat_map(u32::to_le_bytes).collect();
    let cases = [
        (
            "Data\\deflate.bin",
            TestEntry::deflated("Data\\deflate.bin", data.clone()),
        ),
        (
            "Data\\zstd.bin",
            TestEntry::zstd("Data\\zstd.bin", data.clone()),
        ),
        (
            "Data\\plain.bin",
            TestEntry::zstd_without_magic("Data\\plain.bin", data.clone()),
        ),
        (
            "Data\\secret.bin",
            TestEntry::zstd("Data\\secret.bin", data.clone()).encrypted(),
        ),
    ];
    let builder = cases
        .iter()
        .fold(ArchiveBuilder::new(), |builder, (_, entry)| {
            builder.entry(entry.clone())
        });
    let archive = open(dir.path(), &builder)?;
    let target = dir.path().join("out");
    std::fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();

    let report = extract(&archive, &entries, &target, &ExtractOptions::default());

    assert!(report.is_complete(), "every Entry is extracted: {report:?}");
    for (path, _) in cases {
        let file = target.join(path.replace('\\', "/"));
        assert!(
            std::fs::read(&file)? == data,
            "{path} has the original bytes"
        );
    }
    Ok(())
}

fn count_files(dir: &Path) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;
    for item in std::fs::read_dir(dir)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            count += count_files(&item.path())?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}
