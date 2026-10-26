// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::ReadFailure;
use super::tests::{opened, write_archive};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_stopped_build_of_the_tags_stores_nothing_and_a_later_one_builds() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, true)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let source = index
        .forge()
        .read(&index, || {})
        .map_err(ReadFailure::into_reason)?;

    let stopped = source.tags(&|| true).map_err(ReadFailure::into_reason)?;
    assert!(stopped.is_none(), "stopped before it began");
    assert_eq!(index.forge().tag_builds(), 0, "nothing was built");

    let built = source.tags(&|| false).map_err(ReadFailure::into_reason)?;
    assert!(built.is_some(), "built once nobody stops it");
    assert_eq!(index.forge().tag_builds(), 1, "one build");
    let again = source.tags(&|| true).map_err(ReadFailure::into_reason)?;
    assert!(again.is_some(), "a kept tree needs no work, so no stop");
    assert_eq!(index.forge().tag_builds(), 1, "still one build");
    Ok(())
}

#[test]
fn a_consumer_waiting_for_the_build_of_the_tags_stops_waiting_when_cancelled() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, true)?;
    let model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let source = index
        .forge()
        .read(&index, || {})
        .map_err(ReadFailure::into_reason)?;
    let (started, running) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();

    let (waited, built) = thread::scope(|scope| -> Result<_, Box<dyn Error>> {
        let holder = scope.spawn(|| {
            let first = std::cell::Cell::new(true);
            source.tags(&move || {
                if first.replace(false) {
                    let _ = started.send(());
                    let _ = released.recv();
                }
                false
            })
        });
        running.recv_timeout(Duration::from_secs(30))?;
        let waited = source.tags(&|| true);
        let _ = release.send(());
        let built = holder.join().map_err(|_| "the builder panicked")?;
        Ok((waited, built))
    })?;

    assert!(
        matches!(waited, Ok(None)),
        "stopped while waiting for the other build: {waited:?}"
    );
    assert!(
        matches!(built, Ok(Some(_))),
        "the other build was not disturbed: {built:?}"
    );
    Ok(())
}
