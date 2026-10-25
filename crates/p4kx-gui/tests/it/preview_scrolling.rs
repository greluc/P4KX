// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Text and Hex previews scroll with the keyboard once they take the focus.

use crate::waiting::RunQuiet;
use std::error::Error;
use std::time::{Duration, Instant};

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const LINES: usize = 500;
const HEX_ROWS: usize = 400;

fn text() -> String {
    (0..LINES).fold(String::new(), |mut text, line| {
        text.push_str("line ");
        text.push_str(&line.to_string());
        text.push('\n');
        text
    })
}

fn bytes() -> Vec<u8> {
    (0..HEX_ROWS * 16)
        .map(|index| u8::try_from(index % 251).unwrap_or(0))
        .collect()
}

fn hex_row(index: usize) -> String {
    let all = bytes();
    let row = all.get(index * 16..(index + 1) * 16).unwrap_or_default();
    let hex: Vec<String> = row.iter().map(|byte| format!("{byte:02X}")).collect();
    let ascii: String = row
        .iter()
        .map(|&byte| {
            if byte.is_ascii_graphic() || byte == b' ' {
                char::from(byte)
            } else {
                '.'
            }
        })
        .collect();
    format!("{:08X}  {:<47}  {ascii}", index * 16, hex.join(" "))
}

fn opened() -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\lines.txt", text()))
        .entry(TestEntry::stored("Data\\bytes.bin", bytes()))
        .write_to(&path)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((folder, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run_quiet();
    Ok(())
}

fn preview(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })
}

#[test]
fn the_focused_text_view_scrolls_with_the_keyboard() -> TestResult {
    let (_folder, mut harness) = opened()?;
    preview(&mut harness, "lines.txt")?;
    let last = format!("line {}", LINES - 1);
    assert!(harness.query_by_label("line 0").is_some(), "the first line");
    assert!(harness.query_by_label(&last).is_none(), "the last is below");

    harness.get_by_label("Text lines").focus();
    harness.run_quiet();
    harness.key_press(Key::End);
    harness.run_quiet();
    assert!(
        harness.query_by_label(&last).is_some(),
        "End shows the last"
    );
    assert!(
        harness.query_by_label("line 0").is_none(),
        "the first is gone"
    );

    harness.key_press(Key::Home);
    harness.run_quiet();
    assert!(
        harness.query_by_label("line 0").is_some(),
        "Home shows the first"
    );

    harness.key_press(Key::ArrowDown);
    harness.run_quiet();
    assert!(harness.query_by_label("line 0").is_none(), "one line down");
    assert!(harness.query_by_label("line 1").is_some(), "only one line");

    harness.key_press(Key::PageDown);
    harness.run_quiet();
    assert!(harness.query_by_label("line 1").is_none(), "a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run_quiet();
    assert!(
        harness.query_by_label("line 0").is_some(),
        "back at the top"
    );
    Ok(())
}

#[test]
fn a_key_with_a_modifier_does_not_scroll_the_focused_region() -> TestResult {
    let (_folder, mut harness) = opened()?;
    preview(&mut harness, "lines.txt")?;
    harness.get_by_label("Text lines").focus();
    harness.run_quiet();

    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run_quiet();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::PageDown);
    harness.run_quiet();
    assert!(
        harness.query_by_label("line 0").is_some(),
        "Alt and Shift keys leave the region where it is"
    );

    harness.key_press(Key::ArrowDown);
    harness.run_quiet();
    assert!(harness.query_by_label("line 0").is_none(), "a plain key");
    Ok(())
}

#[test]
fn the_focused_hex_view_scrolls_with_the_keyboard() -> TestResult {
    let (_folder, mut harness) = opened()?;
    preview(&mut harness, "bytes.bin")?;
    harness.get_by_label("Hex").click();
    harness.run_quiet();
    let (first, second, last) = (hex_row(0), hex_row(1), hex_row(HEX_ROWS - 1));
    assert!(harness.query_by_label(&first).is_some(), "the first row");
    assert!(harness.query_by_label(&last).is_none(), "the last is below");

    harness.get_by_label("Hexadecimal rows").focus();
    harness.run_quiet();
    harness.key_press(Key::End);
    harness.run_quiet();
    assert!(
        harness.query_by_label(&last).is_some(),
        "End shows the last"
    );
    assert!(
        harness.query_by_label(&first).is_none(),
        "the first is gone"
    );

    harness.key_press(Key::Home);
    harness.run_quiet();
    assert!(
        harness.query_by_label(&first).is_some(),
        "Home shows the first"
    );

    harness.key_press(Key::ArrowDown);
    harness.run_quiet();
    assert!(harness.query_by_label(&first).is_none(), "one row down");
    assert!(harness.query_by_label(&second).is_some(), "only one row");

    harness.key_press(Key::PageDown);
    harness.run_quiet();
    assert!(harness.query_by_label(&second).is_none(), "a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run_quiet();
    assert!(harness.query_by_label(&first).is_some(), "back at the top");
    Ok(())
}
