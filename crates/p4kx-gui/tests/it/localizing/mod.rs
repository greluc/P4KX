// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the localisation tests: a synthetic Archive with `global.ini` files opened in a
//! headless window.

use crate::waiting::wait_for_model;
use std::error::Error;
use std::path::{Path, PathBuf};

use egui::OutputCommand;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, LocalizationStatus, Model, TextLanguage, WorkTab, ui};
use p4kx_testkit::{ArchiveBuilder, GlobalIniBuilder, TestEntry};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) fn english() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("item_Name_Rifle", "Rifle")
        .text("item_Desc_Rifle", "Shoots.\\nWell.")
        .text("vehicle_Name_Aurora", "Aurora MR")
        .text("only_english", "English only")
        .build()
}

pub(crate) fn german() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("ITEM_NAME_RIFLE", "Gewehr")
        .text("item_Desc_Rifle", "Schießt.\\nGut.")
        .text("vehicle_Name_Aurora", "Aurora MR \u{202e}evil\u{1b}")
        .build()
}

pub(crate) fn both() -> Vec<(TextLanguage, Vec<u8>)> {
    vec![
        (TextLanguage::English, english()),
        (TextLanguage::German, german()),
    ]
}

pub(crate) fn archive(
    dir: &Path,
    files: &[(TextLanguage, Vec<u8>)],
) -> Result<PathBuf, Box<dyn Error>> {
    let mut builder = ArchiveBuilder::new().entry(TestEntry::stored("readme.txt", "readme"));
    for (language, bytes) in files {
        builder = builder.entry(TestEntry::zstd(language.global_ini_path(), bytes.clone()));
    }
    let path = dir.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(path)
}

pub(crate) fn harness(model: Model) -> Harness<'static, Model> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    wait_for_model(harness, "the window to settle", done)?;
    harness.run_steps(3);
    Ok(())
}

pub(crate) fn ready(model: &Model) -> bool {
    matches!(
        model.localization().status(),
        LocalizationStatus::Ready(_)
            | LocalizationStatus::NoTexts
            | LocalizationStatus::Failed(_)
            | LocalizationStatus::ArchiveChanged
    )
}

/// Opens the Archive of `files` in a window on the localisation tab and waits until the texts
/// are read.
pub(crate) fn opened(
    files: &[(TextLanguage, Vec<u8>)],
    language: Language,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), files)?;
    let mut harness = harness(Model::new(language));
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Localization);
    harness.run_steps(2);
    wait_until(&mut harness, ready)?;
    Ok((dir, harness))
}

pub(crate) fn copied(harness: &Harness<'_, Model>) -> Vec<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

pub(crate) fn type_search(harness: &mut Harness<'_, Model>, text: &str) -> TestResult {
    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(2);
    harness.get_by_role(Role::TextInput).type_text(text);
    harness.run_steps(2);
    wait_until(harness, |model| !model.localization().is_searching())
}
