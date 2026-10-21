//! Detecting that an open Archive changed on disk, between and during Extractions.

use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use p4kx_extraction::{Archive, ArchiveWatch, ExtractOptions, Progress, extract};
use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn builder() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", "alpha"))
        .entry(TestEntry::stored("Data\\b.txt", "bravo"))
        .entry(TestEntry::stored("Data\\c.txt", "charlie"))
}

fn patched() -> ArchiveBuilder {
    builder().entry(TestEntry::stored("Data\\d.txt", "delta"))
}

fn opened() -> Result<(TempDir, PathBuf, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    write_build_manifest(dir.path(), "sc-alpha-4.3.1", "4.3.1.1", "100", "build-1")?;
    let path = dir.path().join("Data.p4k");
    builder().write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    Ok((dir, path, archive))
}

fn target(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let target = dir.join("out");
    fs::create_dir(&target)?;
    Ok(target)
}

fn set_modified(path: &Path, modified: SystemTime) -> TestResult {
    File::options()
        .write(true)
        .open(path)?
        .set_modified(modified)?;
    Ok(())
}

#[test]
fn an_unchanged_archive_passes_the_check() -> TestResult {
    let (_dir, _path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive);

    watch.check()?;

    assert!(!watch.is_changed(), "nothing changed");
    Ok(())
}

#[test]
fn rewriting_the_archive_with_another_size_is_a_change() -> TestResult {
    let (_dir, path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive);

    patched().write_to(&path)?;

    let error = watch.check().err().ok_or("the change was not noticed")?;
    assert_eq!(error.path(), path, "names the Archive");
    assert!(watch.is_changed(), "the change is remembered");
    Ok(())
}

#[test]
fn a_new_modification_time_alone_is_a_change() -> TestResult {
    let (_dir, path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive);

    set_modified(&path, SystemTime::now() + Duration::from_secs(60))?;

    assert!(watch.check().is_err(), "the timestamp changed");
    Ok(())
}

#[test]
fn a_new_build_alone_is_a_change() -> TestResult {
    let (dir, _path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive);

    write_build_manifest(dir.path(), "sc-alpha-4.3.2", "4.3.2.1", "101", "build-2")?;

    assert!(watch.check().is_err(), "the Build changed");
    Ok(())
}

#[test]
fn a_change_stays_noticed_after_the_file_is_restored() -> TestResult {
    let (_dir, path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive);
    let original = fs::read(&path)?;
    let modified = fs::metadata(&path)?.modified()?;

    patched().write_to(&path)?;
    let first = watch.check();
    fs::write(&path, original)?;
    set_modified(&path, modified)?;

    assert!(first.is_err(), "the change was noticed");
    assert!(watch.check().is_err(), "only a reload clears it");
    Ok(())
}

#[test]
fn the_periodic_check_reads_the_disk_only_after_the_interval() -> TestResult {
    let (_dir, path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive).with_interval(Duration::from_secs(3600));
    watch.poll()?;

    patched().write_to(&path)?;

    assert!(watch.poll().is_ok(), "not due yet");
    assert!(
        watch.check().is_err(),
        "an explicit check always reads the disk"
    );
    Ok(())
}

#[test]
fn an_extraction_of_a_changed_archive_reads_nothing() -> TestResult {
    let (dir, path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive);
    let target = target(dir.path())?;
    let entries: Vec<_> = archive.entries().iter().collect();
    patched().write_to(&path)?;

    let report = extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions {
            watch: Some(&watch),
            ..ExtractOptions::default()
        },
    );

    assert!(report.archive_changed(), "the change is reported");
    assert_eq!(report.extracted(), 0, "nothing is extracted");
    assert_eq!(report.cancelled().len(), 3, "every Entry is cancelled");
    assert!(
        fs::read_dir(&target)?.next().is_none(),
        "nothing is written"
    );
    assert!(!report.is_complete(), "the run is incomplete");
    Ok(())
}

#[test]
fn a_change_during_an_extraction_cancels_the_rest() -> TestResult {
    let (dir, path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive).with_interval(Duration::ZERO);
    let target = target(dir.path())?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let on_progress = |progress: Progress| {
        if progress.entries_done == 1 {
            let _ignored = patched().write_to(&path);
        }
    };
    let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build()?;

    let report = pool.install(|| {
        extract(
            &archive,
            &entries,
            &target,
            &ExtractOptions {
                watch: Some(&watch),
                progress: Some(&on_progress),
                ..ExtractOptions::default()
            },
        )
    });

    assert!(report.archive_changed(), "the change is reported");
    assert_eq!(report.extracted(), 1, "only the Entry before the change");
    assert_eq!(
        report.cancelled(),
        ["Data\\b.txt", "Data\\c.txt"],
        "the rest is cancelled"
    );
    Ok(())
}

#[test]
fn a_change_after_the_last_entry_is_still_reported() -> TestResult {
    let (dir, path, archive) = opened()?;
    let watch = ArchiveWatch::new(&archive).with_interval(Duration::from_secs(3600));
    let target = target(dir.path())?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let on_progress = |progress: Progress| {
        if progress.entries_done == progress.entries_total {
            let _ignored = patched().write_to(&path);
        }
    };

    let report = extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions {
            watch: Some(&watch),
            progress: Some(&on_progress),
            ..ExtractOptions::default()
        },
    );

    assert_eq!(report.extracted(), 3, "every Entry was written before");
    assert!(report.archive_changed(), "the change is reported");
    assert!(!report.is_complete(), "the written data may be stale");
    Ok(())
}

#[test]
fn an_entry_read_while_the_archive_changed_is_not_put_in_place() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let large = ArchiveBuilder::new().entry(TestEntry::stored("Data\\large.bin", vec![7; 4 << 20]));
    large.write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let watch = ArchiveWatch::new(&archive).with_interval(Duration::from_secs(3600));
    watch.check()?;
    let target = target(dir.path())?;
    let entries: Vec<_> = archive.entries().iter().collect();
    let on_progress = |progress: Progress| {
        if progress.entries_done == 0 && progress.bytes_done > 0 {
            let _ignored = set_modified(&path, SystemTime::now() + Duration::from_secs(60));
        }
    };

    let report = extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions {
            watch: Some(&watch),
            progress: Some(&on_progress),
            ..ExtractOptions::default()
        },
    );

    assert_eq!(report.extracted(), 0, "not put in place");
    assert_eq!(
        report.cancelled(),
        ["Data\\large.bin"],
        "cancelled, not failed"
    );
    assert!(report.archive_changed(), "the change is reported");
    assert!(
        fs::read_dir(target.join("Data"))?.next().is_none(),
        "neither the file nor a partial file is left"
    );
    Ok(())
}
