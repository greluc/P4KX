// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Outcome, read, start};
use crate::forge_cache::tests::{opened, write_archive};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_read_that_is_told_to_stop_builds_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, true)?;
    let model = opened(&path)?;
    let index = Arc::clone(model.browser().ok_or("no Archive")?.shared_index());

    let stopped = read(&index, &AtomicBool::new(true));

    assert!(matches!(stopped, Outcome::Failed(None)), "{stopped:?}");
    assert_eq!(index.forge().tag_builds(), 0, "no tree was built");
    assert!(!index.forge().is_read(), "the stopped read is not kept");
    let finished = read(&index, &AtomicBool::new(false));
    assert!(matches!(finished, Outcome::Ready(_)), "{finished:?}");
    assert_eq!(index.forge().tag_builds(), 1, "the next read builds");
    Ok(())
}

#[test]
fn dropping_the_load_tells_the_thread_to_stop() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, true)?;
    let model = opened(&path)?;
    let index = Arc::clone(model.browser().ok_or("no Archive")?.shared_index());

    let load = start(&index, || {})?;
    let stop = Arc::clone(&load.stop);
    assert!(!stop.load(Ordering::Relaxed), "running");
    drop(load);

    assert!(stop.load(Ordering::Relaxed), "stopped by the drop");
    Ok(())
}
