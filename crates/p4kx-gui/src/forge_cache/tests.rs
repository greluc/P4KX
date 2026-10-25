// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

use super::ReadFailure;
use crate::{Language, Model};

type TestResult = Result<(), Box<dyn Error>>;

fn write_archive(path: &Path, extra: bool) -> TestResult {
    let mut forge = DataForgeBuilder::new()
        .structure(TestStruct::new("Maker").field("rank", TestType::Int32))
        .record(
            TestRecord::new(1, "Maker", "Maker.One", "libs/makers/one.xml")
                .value("rank", TestValue::Int32(7)),
        );
    if extra {
        forge = forge.record(TestRecord::new(
            2,
            "Maker",
            "Maker.Two",
            "libs/makers/two.xml",
        ));
    }
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge.build()))
        .write_to(path)?;
    Ok(())
}

fn opened(path: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.to_path_buf(), || {});
    let deadline = Instant::now() + Duration::from_secs(30);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err("the Archive did not open".into());
        }
        thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok(model)
}

#[test]
fn checking_the_archive_does_not_wait_for_a_read_of_dataforge_and_the_read_is_dropped() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let mut model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let (started, running) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();

    let (checked, took, after, again) = thread::scope(|scope| -> Result<_, Box<dyn Error>> {
        let reader_index = &index;
        let reader = scope.spawn(move || {
            reader_index.forge().read(reader_index, || {
                let _ = started.send(());
                let _ = released.recv();
            })
        });
        running.recv_timeout(Duration::from_secs(30))?;
        write_archive(&path, true)?;
        let watchdog = release.clone();
        scope.spawn(move || {
            thread::sleep(Duration::from_secs(3));
            let _ = watchdog.send(());
        });
        let begun = Instant::now();
        let checked = model.check_archive();
        let took = begun.elapsed();
        let _ = release.send(());
        let read = reader.join().map_err(|_| "the reader panicked")?;
        let after = (read, index.forge().is_read());
        let again = (index.forge().read(&index, || {}), index.forge().reads());
        Ok((checked, took, after, again))
    })?;

    assert!(checked, "the change is noticed");
    assert!(
        took < Duration::from_secs(2),
        "check_archive waited for the read: {took:?}"
    );
    assert!(after.0.is_err(), "the read is dropped: {:?}", after.0);
    assert!(!after.1, "nothing is cached");
    assert!(
        matches!(again.0, Err(ReadFailure::ArchiveChanged(_))),
        "the next read stops at the changed Archive: {:?}",
        again.0
    );
    assert_eq!(again.1, 2, "the next read is counted");
    Ok(())
}

#[test]
fn a_cancelled_read_leaves_nothing_in_the_cache_and_the_next_read_works() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());

    let cancelled = index
        .forge()
        .read_until(&index, || {}, &|| true)
        .map_err(ReadFailure::into_reason)?;
    let cached = index.forge().is_read();
    let read = index.forge().read(&index, || {});

    assert!(cancelled.is_none(), "stopped");
    assert!(!cached, "nothing cached");
    assert!(read.is_ok(), "the next read works: {read:?}");
    Ok(())
}

#[test]
fn a_read_that_another_consumer_waits_for_is_not_stopped() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let (started, running) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();

    let (searched, waited) = thread::scope(|scope| -> Result<_, Box<dyn Error>> {
        let shared = &index;
        let searching = scope.spawn(move || {
            shared.forge().read_until(
                shared,
                || {
                    let _ = started.send(());
                    let _ = released.recv();
                },
                &|| true,
            )
        });
        running.recv_timeout(Duration::from_secs(30))?;
        let waiting = scope.spawn(move || shared.forge().read(shared, || {}));
        let deadline = Instant::now() + Duration::from_secs(30);
        while index
            .forge()
            .waiting
            .load(std::sync::atomic::Ordering::Acquire)
            == 0
        {
            if Instant::now() > deadline {
                return Err("the second consumer never waited".into());
            }
            thread::sleep(Duration::from_millis(5));
        }
        let _ = release.send(());
        let searched = searching.join().map_err(|_| "the search panicked")?;
        let waited = waiting.join().map_err(|_| "the waiter panicked")?;
        Ok((searched, waited))
    })?;

    assert!(
        matches!(searched, Ok(Some(_))),
        "the read went on: {searched:?}"
    );
    assert!(waited.is_ok(), "the waiter got it: {waited:?}");
    assert_eq!(index.forge().reads(), 1, "read once");
    Ok(())
}

#[test]
fn a_consumer_waiting_for_the_read_of_another_stops_waiting_when_cancelled() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let (started, running) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();

    let waited = thread::scope(|scope| -> Result<_, Box<dyn Error>> {
        let reader_index = &index;
        let reader = scope.spawn(move || {
            reader_index.forge().read(reader_index, || {
                let _ = started.send(());
                let _ = released.recv();
            })
        });
        running.recv_timeout(Duration::from_secs(30))?;
        let begun = Instant::now();
        let waiting = index
            .forge()
            .read_until(&index, || {}, &|| {
                begun.elapsed() > Duration::from_millis(100)
            })
            .map_err(ReadFailure::into_reason)?;
        let waited = (waiting.is_none(), begun.elapsed());
        let _ = release.send(());
        reader
            .join()
            .map_err(|_| "the reader panicked")?
            .map_err(ReadFailure::into_reason)?;
        Ok(waited)
    })?;

    assert!(waited.0, "gave up without a DataForge");
    assert!(
        waited.1 < Duration::from_secs(5),
        "did not wait for the other read: {:?}",
        waited.1
    );
    Ok(())
}

#[test]
fn a_schema_built_after_the_cache_was_cleared_is_not_stored_for_the_new_state() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let source = index
        .forge()
        .read(&index, || {})
        .map_err(ReadFailure::into_reason)?;

    index.forge().clear();
    let built = source.schema().map_err(ReadFailure::into_reason)?;

    assert!(source.schema_if_built().is_some(), "the old copy keeps it");
    assert!(
        index.forge().schema().is_none(),
        "nothing for the new state"
    );
    assert_eq!(
        index.forge().schema_builds(),
        0,
        "no build is counted for it"
    );
    assert!(!index.forge().is_read(), "the cache stays empty");
    drop(built);
    Ok(())
}

#[test]
fn the_schema_is_built_once_when_threads_ask_at_the_same_moment() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let barrier = std::sync::Barrier::new(8);

    thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                barrier.wait();
                super::ForgeCache::prepare_schema(&index, || {});
            });
        }
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    while index.forge().schema().is_none() {
        if Instant::now() > deadline {
            return Err("the schema was not built".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
    thread::sleep(Duration::from_millis(100));

    assert_eq!(index.forge().schema_builds(), 1, "built once");
    assert_eq!(index.forge().reads(), 1, "read once");
    Ok(())
}

#[test]
fn the_schema_is_tried_once_per_generation_of_the_cache() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    assert!(index.forge().schema_unattempted(), "never tried");

    super::ForgeCache::prepare_schema(&index, || {});
    let deadline = Instant::now() + Duration::from_secs(30);
    while index.forge().schema().is_none() {
        if Instant::now() > deadline {
            return Err("the schema was not built".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!index.forge().schema_unattempted(), "built");

    index.forge().clear();
    assert!(
        index.forge().schema_unattempted(),
        "cleared, so tried again"
    );
    Ok(())
}

#[test]
fn a_stopped_build_of_the_references_stores_nothing_and_a_later_one_builds() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, true)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let source = index
        .forge()
        .read(&index, || {})
        .map_err(ReadFailure::into_reason)?;

    let stopped = source
        .references(&|| true)
        .map_err(ReadFailure::into_reason)?;
    assert!(stopped.is_none(), "stopped before it began");
    assert_eq!(index.forge().reference_builds(), 0, "nothing was built");

    let built = source
        .references(&|| false)
        .map_err(ReadFailure::into_reason)?;
    assert!(built.is_some(), "built once nobody stops it");
    assert_eq!(index.forge().reference_builds(), 1, "one build");
    let again = source
        .references(&|| true)
        .map_err(ReadFailure::into_reason)?;
    assert!(again.is_some(), "a kept index needs no work, so no stop");
    assert_eq!(index.forge().reference_builds(), 1, "still one build");
    Ok(())
}
