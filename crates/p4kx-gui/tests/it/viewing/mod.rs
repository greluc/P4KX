// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the 3D viewer tests: a synthetic Archive with a cube model opened in a headless
//! window.

use std::error::Error;

use crate::waiting::wait_for_model;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, HarnessBuilder};
use p4kx_gui::{Browser, Language, Model, ModelPreview, Preview, ViewerSupport, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry, TestModel};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) fn entries() -> Vec<TestEntry> {
    let cube = TestModel::cube();
    let helpers = TestModel::cube().helpers_only();
    let tower = TestModel::cube().moved([0.0, 0.0, 6.0]);
    vec![
        TestEntry::zstd("Data\\Objects\\cube.cgf", cube.cgf()),
        TestEntry::zstd("Data\\Objects\\cube.cgfm", cube.cgfm()),
        TestEntry::zstd("Data\\Objects\\helpers.cgf", helpers.cgf()),
        TestEntry::zstd("Data\\Objects\\tower.cgf", tower.cgf()),
        TestEntry::zstd("Data\\Objects\\tower.cgfm", tower.cgfm()),
        TestEntry::stored("Data\\Objects\\broken.cgf", "not a chunk file"),
        TestEntry::stored("Data\\readme.txt", "plain text"),
    ]
}

pub(crate) fn opened_with(
    language: Language,
    support: ViewerSupport,
    configure: impl FnOnce(HarnessBuilder<Model>) -> HarnessBuilder<Model>,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let site = Site {
        entries: entries(),
        installation: false,
        key: false,
    };
    opened_at(language, support, site, configure)
}

/// Where and how a test Archive is opened.
pub(crate) struct Site {
    pub(crate) entries: Vec<TestEntry>,
    pub(crate) installation: bool,
    pub(crate) key: bool,
}

pub(crate) fn opened_at(
    language: Language,
    support: ViewerSupport,
    site: Site,
    configure: impl FnOnce(HarnessBuilder<Model>) -> HarnessBuilder<Model>,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = if site.installation {
        let channel = folder.path().join("StarCitizen").join("LIVE");
        std::fs::create_dir_all(channel.join("Bin64"))?;
        channel.join("Data.p4k")
    } else {
        folder.path().join("Data.p4k")
    };
    site.entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    let mut model = Model::new(language);
    model.set_viewer_support(support);
    if site.key {
        model.set_archive_key(Some(p4kx_testkit::test_key()));
    }
    let mut harness = configure(Harness::builder())
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((folder, harness))
}

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    wait_for_model(harness, "the window to settle", done)?;
    harness.run_steps(3);
    Ok(())
}

pub(crate) fn model_preview(model: &Model) -> Option<&ModelPreview> {
    model
        .browser()
        .and_then(Browser::preview)
        .and_then(Preview::model)
}

pub(crate) fn settled(model: &Model) -> bool {
    model_preview(model).is_some_and(|preview| !preview.is_loading())
}

pub(crate) fn select(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, settled)
}
