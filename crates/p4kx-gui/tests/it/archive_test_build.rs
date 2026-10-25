// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! An open Archive of a test Channel: the window shows a persistent warning notice that the
//! material is confidential; the Archive stays browsable.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{ComparisonSide, Language, Model, WorkTab, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn open(channel: &str, language: Language) -> Result<(tempfile::TempDir, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let folder = dir.path().join(channel);
    std::fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", "a"))
        .write_to(&path)?;
    let mut model = Model::new(language);
    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err("timed out opening the Archive".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok((dir, model))
}

fn harness(model: Model) -> Harness<'static, Model> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

#[test]
fn every_test_channel_is_announced_in_british_english() -> TestResult {
    for channel in ["PTU", "EPTU", "HOTFIX", "TECH-PREVIEW"] {
        let (_dir, model) = open(channel, Language::BritishEnglish)?;
        let mut harness = harness(model);

        harness.step();

        harness.get_by_label_contains("which carries Star Citizen test builds");
        harness.get_by_label_contains("is confidential under the Star Citizen Terms of Service");
        harness.get_by_label_contains(&format!("belongs to the {channel} Channel"));
    }
    Ok(())
}

#[test]
fn the_notice_is_also_shown_in_german() -> TestResult {
    let (_dir, model) = open("PTU", Language::German)?;
    let mut harness = harness(model);

    harness.step();

    harness.get_by_label_contains("der Testbuilds von Star Citizen enthält");
    harness.get_by_label_contains("vertraulich");
    Ok(())
}

#[test]
fn the_notice_stays_while_the_window_is_used() -> TestResult {
    let (_dir, model) = open("PTU", Language::BritishEnglish)?;
    let mut harness = harness(model);

    for _ in 0..3 {
        harness.step();
    }

    harness.get_by_label_contains("is confidential");
    Ok(())
}

#[test]
fn live_and_unknown_folders_show_no_notice() -> TestResult {
    for channel in ["LIVE", "Elsewhere"] {
        let (_dir, model) = open(channel, Language::BritishEnglish)?;
        let mut harness = harness(model);

        harness.step();

        assert!(
            harness.query_by_label_contains("test build").is_none(),
            "{channel}: nothing to announce"
        );
    }
    Ok(())
}

fn archive_in(root: &std::path::Path, channel: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let folder = root.join(channel);
    std::fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", "a"))
        .write_to(&path)?;
    Ok(path)
}

fn comparing(old: &str, new: &str) -> Result<(tempfile::TempDir, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut model = Model::new(Language::BritishEnglish);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, archive_in(dir.path(), old)?);
    tab.set_archive(ComparisonSide::New, archive_in(dir.path(), new)?);
    model.set_work_tab(WorkTab::Comparison);
    Ok((dir, model))
}

#[test]
fn a_comparison_of_live_against_ptu_names_the_test_channel() -> TestResult {
    let (_dir, model) = comparing("LIVE", "PTU")?;
    let mut harness = harness(model);

    harness.step();

    harness.get_by_label_contains("belongs to the PTU Channel");
    Ok(())
}

#[test]
fn a_comparison_of_two_test_channels_names_both() -> TestResult {
    let (_dir, model) = comparing("PTU", "EPTU")?;
    let mut harness = harness(model);

    harness.step();

    harness.get_by_label_contains("belong to the PTU, EPTU Channels, which carry");
    assert!(
        harness.query_by_label_contains("belongs to").is_none(),
        "plural form"
    );
    Ok(())
}

#[test]
fn a_comparison_of_live_against_live_shows_no_notice() -> TestResult {
    let (_dir, model) = comparing("LIVE", "Elsewhere")?;
    let mut harness = harness(model);

    harness.step();

    assert!(
        harness.query_by_label_contains("test build").is_none(),
        "nothing to announce"
    );
    Ok(())
}
