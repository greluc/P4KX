// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A headless window over an Archive of the fixtures, and the helpers to drive it.

use crate::waiting::wait_for_model_described;
use std::error::Error;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, KeyText, LineValue, Model, RecordView, TextLanguage, ui};
use tempfile::TempDir;

use super::fixtures::{TestResult, archive};

pub(crate) struct Window {
    pub(crate) _dir: TempDir,
    pub(crate) harness: Harness<'static, Model>,
    pub(crate) settings_changed: Arc<AtomicBool>,
}

pub(crate) fn window(
    interface: p4kx_gui::Language,
    made: Result<(TempDir, PathBuf), Box<dyn Error>>,
) -> Result<Window, Box<dyn Error>> {
    let (dir, path) = made?;
    let settings_changed = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&settings_changed);
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 1000.0])
        .build_ui_state(
            move |ui, model: &mut Model| {
                if ui::show(ui, model).settings_changed {
                    seen.store(true, Ordering::Relaxed);
                }
            },
            Model::new(interface),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    settings_changed.store(false, Ordering::Relaxed);
    Ok(Window {
        _dir: dir,
        harness,
        settings_changed,
    })
}

pub(crate) fn english_window() -> Result<Window, Box<dyn Error>> {
    window(p4kx_gui::Language::BritishEnglish, archive())
}

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    wait_for_model_described(harness, "the window to settle", done, |model| {
        let browser = model.browser();
        let tree = browser.and_then(Browser::records).map(|tree| tree.status());
        let record = browser
            .and_then(Browser::record_preview)
            .map(|preview| preview.status());
        format!("tree {tree:?}, preview {record:?}")
    })?;
    harness.run_steps(2);
    Ok(())
}

pub(crate) fn preview(model: &Model) -> Option<&RecordView> {
    model
        .browser()
        .and_then(Browser::record_preview)
        .and_then(|preview| preview.view())
}

pub(crate) fn read_in(language: Option<TextLanguage>) -> impl Fn(&Model) -> bool {
    move |model| preview(model).is_some_and(|view| view.language() == language)
}

pub(crate) fn open_record(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label("Expand DataForge").click();
    wait_until(harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| tree.rows().len() > 1)
    })?;
    harness.get_by_label("Expand libs").click();
    harness.run_steps(2);
    harness.get_by_label("Expand vehicles").click();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, name).click();
    wait_until(harness, |model| preview(model).is_some())
}

pub(crate) fn choose(harness: &mut Harness<'_, Model>, entry: &str) {
    harness.get_by_label("Record texts").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::RadioButton, entry)
        .click();
    harness.run_steps(2);
}

pub(crate) fn has_row(harness: &Harness<'_, Model>, label: &str) -> bool {
    harness.query_by_role_and_label(Role::Row, label).is_some()
}

pub(crate) fn key_text<'v>(
    view: &'v RecordView,
    name: &str,
) -> Option<(&'v str, &'v [Range<usize>])> {
    match key_line(view, name)? {
        KeyText::Text { text, escapes } => Some((text, escapes)),
        _ => None,
    }
}

pub(crate) fn key_line<'v>(view: &'v RecordView, name: &str) -> Option<&'v KeyText> {
    view.lines()
        .iter()
        .find(|line| line.name() == name)
        .and_then(|line| match line.value() {
            LineValue::Key { text, .. } => Some(text),
            _ => None,
        })
}
