// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The lists of the Extraction summary scroll with the keyboard once they take the focus.

use std::error::Error;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Extraction, ExtractionStatus, Language, Model, ui};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};
use tempfile::TempDir;

use crate::waiting::{RunQuiet, wait_for_model};

type TestResult = Result<(), Box<dyn Error>>;

const COUNT: u128 = 30;

fn database() -> Vec<u8> {
    (1..=COUNT)
        .fold(
            DataForgeBuilder::new()
                .structure(TestStruct::new("Item").field("label", TestType::String)),
            |builder, n| {
                builder.record(
                    TestRecord::new(
                        n,
                        "Item",
                        format!("item{n:02}"),
                        format!("../evil{n:02}.xml"),
                    )
                    .value("label", TestValue::text("Evil")),
                )
            },
        )
        .build()
}

fn finished(size: [f32; 2]) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    (0..COUNT)
        .fold(ArchiveBuilder::new(), |builder, n| {
            builder.entry(TestEntry::stored(format!("Data\\dir{n:02}\\NUL"), "device"))
        })
        .entry(TestEntry::stored("Data\\Game2.dcb", database()))
        .write_to(&path)?;
    let out = dir.path().join("out");
    std::fs::create_dir(&out)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_extraction_folder(Some(out));
    model.set_extraction_conversion(true);
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    wait_for_model(&mut harness, "the Archive", |model| {
        model.browser().is_some()
    })?;
    harness.get_by_label("Extract shown…").click();
    harness.run_steps(2);
    harness.get_by_label("Existing folder").click();
    harness.run_steps(2);
    harness.get_by_label("Extract").click();
    harness.run_steps(2);
    harness.get_by_label("Extract whole Archive").click();
    wait_for_model(&mut harness, "the Extraction summary", |model| {
        model
            .extraction()
            .is_some_and(|extraction| !extraction.is_running())
    })?;
    harness.run_steps(3);
    Ok((dir, harness))
}

fn lines(model: &Model) -> Result<(Vec<String>, Vec<String>), Box<dyn Error>> {
    let Some(ExtractionStatus::Finished(summary)) = model.extraction().map(Extraction::status)
    else {
        return Err("no summary".into());
    };
    let failed = summary
        .failures()
        .iter()
        .map(|failure| format!("{}: {}", failure.path, failure.reason))
        .collect();
    let records = summary
        .record_failures()
        .iter()
        .map(|failure| {
            format!(
                "{} ({}): {}",
                failure.record_path, failure.guid, failure.reason
            )
        })
        .collect();
    Ok((failed, records))
}

fn scrolls_with_the_keyboard(
    harness: &mut Harness<'_, Model>,
    (region, rows): (&str, &[String]),
) -> TestResult {
    let (Some(first), Some(second), Some(last)) = (rows.first(), rows.get(1), rows.last()) else {
        return Err("too few rows".into());
    };
    let shown = |harness: &Harness<'_, Model>, row: &String| harness.query_by_label(row).is_some();
    assert!(shown(harness, first), "{region}: the first row");
    assert!(!shown(harness, last), "{region}: the last is below");

    harness
        .get_by_role_and_label(Role::ScrollView, region)
        .focus();
    harness.run_quiet();
    harness.key_press(Key::End);
    harness.run_quiet();
    assert!(shown(harness, last), "{region}: End shows the last");
    assert!(!shown(harness, first), "{region}: the first is gone");

    harness.key_press(Key::Home);
    harness.run_quiet();
    assert!(shown(harness, first), "{region}: Home shows the first");

    harness.key_press(Key::ArrowDown);
    harness.run_quiet();
    assert!(!shown(harness, first), "{region}: one row down");
    assert!(shown(harness, second), "{region}: only one row");

    harness.key_press(Key::PageDown);
    harness.run_quiet();
    assert!(!shown(harness, second), "{region}: a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run_quiet();
    assert!(shown(harness, first), "{region}: back at the top");

    harness.key_press_modifiers(Modifiers::ALT, Key::End);
    harness.run_quiet();
    assert!(shown(harness, first), "{region}: a modified key stays out");
    Ok(())
}

#[test]
fn the_failed_entries_scroll_with_the_keyboard() -> TestResult {
    let (_dir, mut harness) = finished([1400.0, 900.0])?;
    let (failed, _) = lines(harness.state())?;
    assert_eq!(failed.len(), usize::try_from(COUNT)?, "every Entry failed");
    scrolls_with_the_keyboard(&mut harness, ("Failed (30), rows", &failed))
}

#[test]
fn the_records_not_written_scroll_with_the_keyboard() -> TestResult {
    let (_dir, mut harness) = finished([1400.0, 900.0])?;
    let (_, records) = lines(harness.state())?;
    assert_eq!(
        records.len(),
        usize::try_from(COUNT)?,
        "every Record failed"
    );
    scrolls_with_the_keyboard(&mut harness, ("Records not written (30), rows", &records))
}
