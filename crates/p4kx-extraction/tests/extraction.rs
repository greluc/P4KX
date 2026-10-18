//! Running an Extraction through the public API.

use std::error::Error;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use p4kx_extraction::{Archive, ExtractionError, UnknownEntries, extract, select_entries};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn open(dir: &Path, builder: &ArchiveBuilder) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(Archive::open(&path)?)
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
fn extracts_entries_and_reports_counts_and_failures() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path(), &sample())?;
    let target = dir.path().join("out");
    std::fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();

    let report = extract(&archive, &entries, &target, &AtomicBool::new(false));

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

    let report = extract(&archive, &entries, &target, &AtomicBool::new(false));

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

    let report = extract(&archive, &entries, &target, &AtomicBool::new(false));

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
fn a_cancelled_run_counts_the_remaining_entries_as_cancelled() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path(), &sample())?;
    let target = dir.path().join("out");
    std::fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();

    let report = extract(&archive, &entries, &target, &AtomicBool::new(true));

    assert_eq!(
        report.cancelled(),
        entries.len(),
        "all Entries are cancelled"
    );
    assert_eq!(report.extracted(), 0, "nothing is extracted");
    assert!(!report.is_complete(), "a cancelled run is incomplete");
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

    let report = extract(&archive, &entries, &target, &AtomicBool::new(false));

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
