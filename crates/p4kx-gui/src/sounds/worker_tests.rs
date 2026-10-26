// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc;

use p4kx_extraction::{Archive, CancelToken};
use p4kx_testkit::{ArchiveBuilder, TestAtl, TestEntry, test_key};

use super::worker::{Answer, Request, Worker, run};
use crate::EntryIndex;

type TestResult = Result<(), Box<dyn Error>>;

fn opened() -> Result<(tempfile::TempDir, Arc<EntryIndex>), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\GameAudio\\SC_Audio.xml",
            TestAtl::new("SC_Audio")
                .trigger("Play_It", &["Play_It"])
                .to_xml(),
        ))
        .write_to(&path)?;
    let index = EntryIndex::build(Archive::open_with_key(&path, Some(test_key()))?);
    Ok((folder, Arc::new(index)))
}

#[test]
fn a_cancelled_worker_stops_indexing_and_answers_nothing() -> TestResult {
    let (_folder, index) = opened()?;
    let (_requests, waiting) = mpsc::channel::<Request>();
    let (answer, answers) = mpsc::channel::<Answer>();
    let cancel = CancelToken::new();
    cancel.cancel();

    run(&index, (&waiting, &answer), &cancel, &|| {});

    assert!(
        answers.try_recv().is_err(),
        "no answer from a cancelled build"
    );
    Ok(())
}

#[test]
fn an_uncancelled_worker_answers_that_the_index_is_ready() -> TestResult {
    let (_folder, index) = opened()?;
    let (requests, waiting) = mpsc::channel::<Request>();
    let (answer, answers) = mpsc::channel::<Answer>();
    drop(requests);

    run(&index, (&waiting, &answer), &CancelToken::new(), &|| {});

    assert!(
        matches!(answers.try_recv(), Ok(Answer::Ready(info)) if info.triggers == 1),
        "the index is ready"
    );
    Ok(())
}

#[test]
fn dropping_the_worker_cancels_its_indexing() -> TestResult {
    let (_folder, index) = opened()?;
    let worker = Worker::start(&index, Arc::new(|| {}))?;
    let token = worker.cancel_token();

    assert!(!token.is_cancelled(), "running");
    drop(worker);

    assert!(token.is_cancelled(), "stopped with its handle");
    Ok(())
}
