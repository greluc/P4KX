// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Writing a model as GLB with progress and cancellation.

use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{AnimationSet, CancelToken, ConversionError, GlbOptions, ModelSource};
use p4kx_testkit::TestModel;

type TestResult = Result<(), Box<dyn Error>>;

fn source(folder: &Path) -> Result<ModelSource, Box<dyn Error>> {
    let cube = TestModel::cube();
    fs::write(folder.join("cube.cgf"), cube.cgf())?;
    fs::write(folder.join("cube.cgfm"), cube.cgfm())?;
    Ok(ModelSource::from_file(&folder.join("cube.cgf"))?)
}

#[test]
fn progress_counts_the_bytes_against_the_length_of_the_file() -> TestResult {
    let folder = tempfile::tempdir()?;
    let source = source(folder.path())?;
    let out = folder.path().join("cube.glb");
    let seen = RefCell::new(Vec::new());

    let summary = source.write_glb_observed(
        &AnimationSet::default(),
        &GlbOptions::default(),
        &out,
        None,
        &CancelToken::new(),
        &|done, total| seen.borrow_mut().push((done, total)),
    )?;

    let seen = seen.into_inner();
    let last = seen.last().copied().ok_or("no progress")?;
    assert_eq!(last, (summary.bytes, summary.bytes), "ends at the length");
    assert_eq!(fs::metadata(&out)?.len(), summary.bytes, "the file");
    assert!(
        seen.windows(2).all(|pair| match pair {
            [(before, _), (after, _)] => before <= after,
            _ => true,
        }),
        "never goes back"
    );
    assert!(
        seen.iter().all(|&(done, total)| done <= total),
        "never beyond the total"
    );
    Ok(())
}

#[test]
fn a_cancelled_export_writes_nothing() -> TestResult {
    let folder = tempfile::tempdir()?;
    let source = source(folder.path())?;
    let out = folder.path().join("cube.glb");
    let cancel = CancelToken::new();
    cancel.cancel();

    let result = source.write_glb_observed(
        &AnimationSet::default(),
        &GlbOptions::default(),
        &out,
        None,
        &cancel,
        &|_, _| {},
    );

    assert!(
        matches!(result, Err(ConversionError::Cancelled)),
        "cancelled: {result:?}"
    );
    assert!(!out.exists(), "no file");
    assert_eq!(parts(folder.path())?, 0, "no temporary file");
    Ok(())
}

fn parts(folder: &Path) -> Result<usize, std::io::Error> {
    Ok(fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|part| part == "part"))
        .count())
}

#[test]
fn a_cancel_during_the_write_removes_the_temporary_file() -> TestResult {
    let folder = tempfile::tempdir()?;
    let source = source(folder.path())?;
    let out = folder.path().join("cube.glb");
    let cancel = CancelToken::new();

    let result = source.write_glb_observed(
        &AnimationSet::default(),
        &GlbOptions::default(),
        &out,
        None,
        &cancel,
        &|_, _| {
            cancel.cancel();
        },
    );

    assert!(
        matches!(result, Err(ConversionError::Cancelled)),
        "the write stops: {result:?}"
    );
    assert!(!out.exists(), "no file");
    assert_eq!(parts(folder.path())?, 0, "no temporary file");
    Ok(())
}
