// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Extracting the changes of one side of a Comparison of two real Archives; runs with
//! `--ignored` when `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B` name them, and skips without the second.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{
    ComparisonSide, ComparisonStatus, Extraction, ExtractionScope, ExtractionStatus, Language,
    Model, TargetKind,
};
use p4kx_testkit::real_archive_pair;

type TestResult = Result<(), Box<dyn Error>>;

const RUN_BUDGET: Duration = Duration::from_secs(120);
const FRAME_LIMIT: Duration = Duration::from_millis(16);

fn poll_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + RUN_BUDGET;
    while !done(model) {
        if Instant::now() > deadline {
            return Err(format!("did not settle: {:?}", model.comparison().status()).into());
        }
        std::thread::sleep(Duration::from_millis(20));
        model.comparison_mut().poll();
        model.poll_extraction();
    }
    Ok(())
}

fn settled(model: &Model) -> bool {
    matches!(model.comparison().status(), ComparisonStatus::Finished(view) if !view.is_filtering())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_changes_of_a_real_side_start_without_blocking_and_account_for_every_entry() -> TestResult {
    let Some((old, new)) = real_archive_pair()? else {
        return Ok(());
    };
    let out = tempfile::tempdir()?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old);
    tab.set_archive(ComparisonSide::New, new);
    assert!(tab.start(|| {}), "started");
    poll_until(&mut model, settled)?;
    let scope = ExtractionScope::Changes(ComparisonSide::New);
    let selected = model.scope_len(scope);

    model.begin_extraction(scope);
    let setup = model.extraction_setup_mut().ok_or("no setup")?;
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out.path().to_path_buf());
    let submitted_at = Instant::now();
    let started = model.submit_extraction(|| {});
    let submitting = submitted_at.elapsed();
    model.cancel_extraction();
    poll_until(&mut model, |model| {
        !model.extraction().is_some_and(Extraction::is_running)
    })?;

    assert!(selected > 0 || !started, "a real patch changes Entries");
    assert!(submitting < FRAME_LIMIT, "submitting took {submitting:?}");
    if let Some(ExtractionStatus::Finished(summary)) = model.extraction().map(Extraction::status) {
        let accounted = summary.extracted()
            + summary.skipped().len()
            + summary.failures().len()
            + summary.cancelled().len();
        assert_eq!(accounted, selected, "every selected Entry is accounted for");
    } else {
        return Err("no summary".into());
    }
    Ok(())
}
