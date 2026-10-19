//! Helpers of the Record preview model tests: the Archive opened headless, Records selected and
//! their preview waited for.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Browser, Language, Model, RecordPreview, RecordPreviewStatus, RecordTree, RecordTreeStatus,
};
use tempfile::TempDir;

use super::previewing_records::{TestResult, archive, database};

/// Opens the Archive of [`database`] and loads its DataForge.
pub(crate) fn opened() -> Result<(TempDir, PathBuf, Model), Box<dyn Error>> {
    let (dir, path) = archive(&database())?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.clone(), || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    model.browser_mut().ok_or("no Archive")?.load_records(|| {});
    wait_until(&mut model, |model| {
        tree(model).is_ok_and(|tree| matches!(tree.status(), RecordTreeStatus::Ready(_)))
    })?;
    Ok((dir, path, model))
}

/// Polls `model` until `done` holds.
pub(crate) fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the model did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        model.query_mut().poll();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

/// Returns the Record tree.
pub(crate) fn tree(model: &Model) -> Result<&RecordTree, Box<dyn Error>> {
    model
        .browser()
        .and_then(Browser::records)
        .ok_or_else(|| "no DataForge node".into())
}

/// Returns the record path of the Record selected in the tree.
pub(crate) fn selected_path(model: &Model) -> Option<String> {
    let browser = model.browser()?;
    let id = browser.selected_record()?;
    Some(browser.records()?.record(id)?.record_path().to_owned())
}

/// Selects the Record at `path` in the tree as a click on its row does.
pub(crate) fn select(model: &mut Model, path: &str) -> TestResult {
    let node = tree(model)?
        .find(path)
        .ok_or_else(|| format!("no node {path}"))?;
    model
        .browser_mut()
        .ok_or("no Archive")?
        .select_record_node(node, || {});
    Ok(())
}

/// Asks for the preview of the selected Record and waits until it was read.
pub(crate) fn previewed(model: &mut Model) -> Result<&RecordPreview, Box<dyn Error>> {
    model
        .browser_mut()
        .ok_or("no Archive")?
        .load_record_preview(|| {});
    wait_until(model, |model| {
        model
            .browser()
            .and_then(Browser::record_preview)
            .is_some_and(|preview| !matches!(preview.status(), RecordPreviewStatus::Loading))
    })?;
    model
        .browser()
        .and_then(Browser::record_preview)
        .ok_or_else(|| "no Record preview".into())
}
