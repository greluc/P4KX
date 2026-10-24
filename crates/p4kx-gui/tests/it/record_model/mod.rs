// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Comparing a pair of Archives on the model, without a window, and reaching its Record list.

use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use p4kx_gui::{
    ComparisonLevels, ComparisonSide, ComparisonStatus, ComparisonTab, ComparisonView, Language,
    Model, RecordChangeView,
};

use crate::record_pair::TestResult;

/// Polls `tab` until `done` holds.
pub(crate) fn wait(tab: &mut ComparisonTab, done: impl Fn(&ComparisonTab) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(tab) {
        if Instant::now() > deadline {
            return Err(format!("the Comparison did not settle: {:?}", tab.status()).into());
        }
        std::thread::sleep(Duration::from_millis(5));
        tab.poll();
    }
    Ok(())
}

/// Compares `old` with `new` on `levels` and waits until it finished.
pub(crate) fn compared(
    (old, new): (&Path, &Path),
    levels: ComparisonLevels,
) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old.to_path_buf());
    tab.set_archive(ComparisonSide::New, new.to_path_buf());
    tab.set_levels(levels);
    if !tab.start(|| {}) {
        return Err("the Comparison did not start".into());
    }
    wait(tab, |tab| !tab.is_running())?;
    match tab.status() {
        ComparisonStatus::Finished(_) => Ok(model),
        other => Err(format!("not finished: {other:?}").into()),
    }
}

pub(crate) fn records(model: &Model) -> Result<&RecordChangeView, Box<dyn Error>> {
    model
        .comparison()
        .view()
        .and_then(ComparisonView::records)
        .ok_or_else(|| "no Record list".into())
}

pub(crate) fn records_mut(model: &mut Model) -> Result<&mut RecordChangeView, Box<dyn Error>> {
    model
        .comparison_mut()
        .view_mut()
        .and_then(ComparisonView::records_mut)
        .ok_or_else(|| "no Record list".into())
}
