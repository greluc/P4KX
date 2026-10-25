// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the Query model tests: an Archive opened headless and Queries run to the end.

use crate::waiting::{wait_polling, wait_polling_described};
use std::error::Error;
use std::path::Path;

use p4kx_gui::{Language, Model, QueryResult, QueryStatus};

use super::TestResult;
/// Opens the Archive at `path` in a headless model.
pub(crate) fn opened(path: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.to_path_buf(), || {});
    wait_polling_described(
        &mut model,
        "the Archive to open",
        |model| {
            model.poll();
        },
        |model| model.browser().is_some(),
        |model| format!("failure {:?}", model.failure()),
    )?;
    Ok(model)
}

/// Waits until the Query of the tab ended.
pub(crate) fn settled(model: &mut Model) -> TestResult {
    wait_polling(
        model,
        "the Query to end",
        |model| {
            model.query_mut().poll();
        },
        |model| !matches!(model.query().status(), QueryStatus::Running(_)),
    )?;
    Ok(())
}

/// Runs `text` in the Query tab of `model` and waits until it ended.
pub(crate) fn run(model: &mut Model, text: &str) -> TestResult {
    model.query_mut().set_text(text);
    model.run_query(|| {});
    settled(model)
}

pub(crate) fn finished(model: &Model) -> Result<&QueryResult, String> {
    match model.query().status() {
        QueryStatus::Finished(result) => Ok(result),
        other => Err(format!("not finished: {other:?}")),
    }
}

pub(crate) fn column(result: &QueryResult, column: usize) -> Vec<String> {
    result
        .page_rows()
        .filter_map(|row| result.cell(row, column).map(str::to_owned))
        .collect()
}
