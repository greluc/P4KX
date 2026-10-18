//! Progress, cancellation and parallelism of an Extraction.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use p4kx_extraction::{
    Archive, CancelToken, ConflictPolicy, ExtractOptions, ExtractionReport, Progress, extract,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const LARGE_LEN: usize = 8 << 20;

fn open(dir: &Path, builder: &ArchiveBuilder) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(Archive::open(&path)?)
}

fn large_data() -> Vec<u8> {
    (0..LARGE_LEN)
        .map(|index| u8::try_from(index % 251).unwrap_or_default())
        .collect()
}

fn files_below(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            files.extend(files_below(&item.path())?);
        } else {
            files.push(item.path());
        }
    }
    files.sort();
    Ok(files)
}

#[test]
fn a_run_cancelled_before_it_starts_reports_every_entry_as_cancelled() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new()
            .entry(TestEntry::stored("Data\\a.txt", b"a".to_vec()))
            .entry(TestEntry::stored("Data\\b.txt", b"b".to_vec())),
    )?;
    let target = dir.path().join("out");
    fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let cancel = CancelToken::new();
    cancel.cancel();

    let report = extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions {
            cancel: Some(&cancel),
            ..ExtractOptions::default()
        },
    );

    assert_eq!(
        report.cancelled(),
        ["Data\\a.txt", "Data\\b.txt"],
        "every Entry is cancelled"
    );
    assert_eq!(report.extracted(), 0, "nothing is extracted");
    assert!(files_below(&target)?.is_empty(), "nothing is written");
    assert!(!report.is_complete(), "a cancelled run is incomplete");
    Ok(())
}

#[test]
fn cancelling_inside_an_entry_stops_it_and_leaves_no_partial_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(TestEntry::deflated("Data\\large.bin", large_data())),
    )?;
    let target = dir.path().join("out");
    fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let cancel = CancelToken::new();
    let on_progress = |progress: Progress| {
        if progress.bytes_done > 0 {
            cancel.cancel();
        }
    };

    let report = extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions {
            cancel: Some(&cancel),
            progress: Some(&on_progress),
            ..ExtractOptions::default()
        },
    );

    assert_eq!(
        report.cancelled(),
        ["Data\\large.bin"],
        "the Entry in progress is cancelled"
    );
    assert!(report.failures().is_empty(), "cancelling is not a failure");
    assert!(
        files_below(&target)?.is_empty(),
        "no partial file is left: {:?}",
        files_below(&target)?
    );
    Ok(())
}

#[test]
fn cancelling_an_overwrite_keeps_the_existing_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(TestEntry::stored("Data\\large.bin", large_data())),
    )?;
    let target = dir.path().join("out");
    fs::create_dir_all(target.join("Data"))?;
    fs::write(target.join("Data").join("large.bin"), "mine")?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let cancel = CancelToken::new();
    let on_progress = |progress: Progress| {
        if progress.bytes_done > 0 {
            cancel.cancel();
        }
    };

    let report = extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions {
            policy: ConflictPolicy::Overwrite,
            cancel: Some(&cancel),
            progress: Some(&on_progress),
            watch: None,
        },
    );

    assert_eq!(
        report.cancelled(),
        ["Data\\large.bin"],
        "the Entry is cancelled"
    );
    assert_eq!(
        fs::read_to_string(target.join("Data").join("large.bin"))?,
        "mine",
        "the existing file is unchanged"
    );
    assert_eq!(files_below(&target)?.len(), 1, "no temporary file is left");
    Ok(())
}

#[test]
fn progress_ends_with_every_entry_and_byte_done() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new()
            .entry(TestEntry::zstd("Data\\large.bin", large_data()))
            .entry(TestEntry::stored("Data\\small.txt", b"small".to_vec()))
            .entry(
                TestEntry::deflated("Data\\broken.xml", b"<b/>".to_vec())
                    .with_payload(vec![0xFF; 8]),
            ),
    )?;
    let target = dir.path().join("out");
    fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let seen = Mutex::new(Vec::new());
    let on_progress = |progress: Progress| {
        if let Ok(mut seen) = seen.lock() {
            seen.push(progress);
        }
    };

    let _report = extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions {
            progress: Some(&on_progress),
            ..ExtractOptions::default()
        },
    );

    let seen = seen.into_inner()?;
    let total_bytes = LARGE_LEN as u64 + 5 + 4;
    assert!(
        seen.iter()
            .all(|progress| progress.entries_total == 3 && progress.bytes_total == total_bytes),
        "every report carries the totals"
    );
    assert!(
        seen.len() > 3,
        "a large Entry reports progress while it is written: {} reports",
        seen.len()
    );
    assert!(never_goes_back(&seen), "progress never goes back");
    assert_eq!(
        seen.iter().map(|progress| progress.entries_done).max(),
        Some(3),
        "every Entry is counted once it ends, failed ones included"
    );
    assert_eq!(
        seen.iter().map(|progress| progress.bytes_done).max(),
        Some(total_bytes),
        "every byte is counted once its Entry ends"
    );
    Ok(())
}

#[test]
fn a_parallel_run_writes_the_same_files_and_report_as_a_sequential_run() -> TestResult {
    let dir = tempfile::tempdir()?;
    let builder = (0..200)
        .fold(ArchiveBuilder::new(), |builder, index| {
            let data = format!("entry {index}").repeat(index + 1).into_bytes();
            let entry = match index % 3 {
                0 => TestEntry::stored(format!("Data\\{}\\{index}.txt", index % 7), data),
                1 => TestEntry::deflated(format!("Data\\{}\\{index}.txt", index % 7), data),
                _ => TestEntry::zstd(format!("Data\\{}\\{index}.txt", index % 7), data).encrypted(),
            };
            builder.entry(entry)
        })
        .entry(TestEntry::stored("Data\\dup.txt", b"first".to_vec()))
        .entry(TestEntry::stored("data\\DUP.txt", b"second".to_vec()))
        .entry(TestEntry::stored("Data\\clash", b"file first".to_vec()))
        .entry(TestEntry::stored(
            "Data\\Clash\\inner.txt",
            b"needs a folder".to_vec(),
        ))
        .entry(TestEntry::stored(
            "Data\\Nest\\inner.txt",
            b"folder first".to_vec(),
        ))
        .entry(TestEntry::stored("data\\nest", b"needs no folder".to_vec()))
        .entry(
            TestEntry::deflated("Data\\broken.xml", b"<b/>".to_vec()).with_payload(vec![0xFF; 8]),
        );
    let archive = open(dir.path(), &builder)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let sequential_target = dir.path().join("sequential");
    let parallel_target = dir.path().join("parallel");
    fs::create_dir(&sequential_target)?;
    fs::create_dir(&parallel_target)?;
    let options = ExtractOptions {
        policy: ConflictPolicy::Overwrite,
        ..ExtractOptions::default()
    };

    let sequential = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()?
        .install(|| extract(&archive, &entries, &sequential_target, &options));
    let parallel = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()?
        .install(|| extract(&archive, &entries, &parallel_target, &options));

    assert_eq!(summary(&parallel), summary(&sequential), "same report");
    assert_eq!(
        tree(&parallel_target)?,
        tree(&sequential_target)?,
        "same files with the same content"
    );
    Ok(())
}

fn never_goes_back(seen: &[Progress]) -> bool {
    seen.windows(2).all(|pair| match pair {
        [before, after] => {
            before.entries_done <= after.entries_done && before.bytes_done <= after.bytes_done
        }
        _ => true,
    })
}

type Tree = Vec<(PathBuf, Vec<u8>)>;

fn tree(root: &Path) -> Result<Tree, Box<dyn Error>> {
    files_below(root)?
        .into_iter()
        .map(|file| {
            let content = fs::read(&file)?;
            Ok((file.strip_prefix(root)?.to_path_buf(), content))
        })
        .collect()
}

fn summary(report: &ExtractionReport) -> (usize, Vec<String>, Vec<String>, Vec<String>) {
    (
        report.extracted(),
        report.skipped().to_vec(),
        report
            .failures()
            .iter()
            .map(|failure| failure.path().to_owned())
            .collect(),
        report.cancelled().to_vec(),
    )
}
