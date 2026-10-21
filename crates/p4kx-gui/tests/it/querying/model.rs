//! Helpers of the Query model tests: an Archive opened headless and Queries run to the end.

use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use p4kx_gui::{Language, Model, QueryResult, QueryStatus};

use super::TestResult;
/// Opens the Archive at `path` in a headless model.
pub(crate) fn opened(path: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.to_path_buf(), || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err(format!("not opened: {:?}", model.failure()).into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok(model)
}

/// Waits until the Query of the tab ended.
pub(crate) fn settled(model: &mut Model) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    while matches!(model.query().status(), QueryStatus::Running(_)) {
        if Instant::now() > deadline {
            return Err("the Query did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.query_mut().poll();
    }
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
