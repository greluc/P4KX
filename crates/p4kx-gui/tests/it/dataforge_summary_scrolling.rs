// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The summary of the DataForge tab scrolls with the keyboard once it takes the focus.

use std::error::Error;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, Language, Model, RecordTreeStatus, SUMMARY_RECORDS, ui};
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct};
use tempfile::TempDir;

use crate::waiting::{RunQuiet, wait_for_model};

type TestResult = Result<(), Box<dyn Error>>;

const REGION: &str = "DataForge summary";

fn name(index: usize) -> String {
    format!("Item.N{index:02}")
}

fn database() -> Vec<u8> {
    (0..SUMMARY_RECORDS)
        .fold(
            DataForgeBuilder::new().structure(TestStruct::new("Item")),
            |builder, n| {
                builder.record(TestRecord::new(
                    u128::try_from(n).unwrap_or_default() + 1,
                    "Item",
                    name(n),
                    format!("libs/items/item{n:02}.xml"),
                ))
            },
        )
        .build()
}

fn shown(size: [f32; 2]) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database()))
        .write_to(&path)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
    harness.state_mut().open(path, || {});
    wait_for_model(&mut harness, "the Archive", |model| {
        model.browser().is_some()
    })?;
    harness.run_quiet();
    harness
        .get_by_role_and_label(Role::Button, "DataForge")
        .click();
    wait_for_model(&mut harness, "the DataForge summary", |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| matches!(tree.status(), RecordTreeStatus::Ready(_)))
    })?;
    harness.run_quiet();
    Ok((dir, harness))
}

fn top(harness: &Harness<'_, Model>, label: &str) -> f32 {
    harness.get_by_label(label).rect().min.y
}

#[test]
fn the_focused_summary_scrolls_with_the_keyboard() -> TestResult {
    let (_dir, mut harness) = shown([1600.0, 560.0])?;
    let last = name(SUMMARY_RECORDS - 1);
    let start = top(&harness, &last);

    harness
        .get_by_role_and_label(Role::ScrollView, REGION)
        .focus();
    harness.run_quiet();
    assert!(
        (top(&harness, &last) - start).abs() < 0.5,
        "focusing does not move the summary"
    );
    harness.key_press(Key::End);
    harness.run_quiet();
    let end = top(&harness, &last);
    assert!(end < start, "End scrolls down: {end} against {start}");

    harness.key_press(Key::Home);
    harness.run_quiet();
    assert!(
        (top(&harness, &last) - start).abs() < 0.5,
        "Home returns to the top"
    );

    harness.key_press(Key::ArrowDown);
    harness.run_quiet();
    let one = start - top(&harness, &last);
    assert!(one > 0.0, "an arrow scrolls down");

    harness.key_press(Key::PageDown);
    harness.run_quiet();
    assert!(
        start - top(&harness, &last) > one,
        "a page scrolls further than an arrow"
    );

    harness.key_press(Key::Home);
    harness.key_press_modifiers(Modifiers::ALT, Key::End);
    harness.run_quiet();
    assert!(
        (top(&harness, &last) - start).abs() < 0.5,
        "a modified key stays out"
    );
    Ok(())
}
