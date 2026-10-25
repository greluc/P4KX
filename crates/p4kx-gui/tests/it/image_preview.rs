// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Image tab of the preview over a synthetic Archive with split textures, driven headless.

use crate::imaging;
use crate::waiting::{RunQuiet, wait_polling};

use std::cell::Cell;

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use imaging::{
    TestResult, archive, image, opened, select, settled, shown_level, textures, wait_until,
};
use p4kx_gui::{Browser, ImagePreview, ImageStatus, Language, MipStatus, Model, PreviewTab};
const PREVIEW_PADDING: f32 = 16.0;
const PREVIEW_PADDING_Y: f32 = 8.0;
const SELECTOR_GAP: f32 = 8.0;

#[test]
fn selecting_a_texture_shows_the_largest_mip_that_fits_the_view() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "panel_diff.dds")?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Image,
        "Image tab"
    );
    let level = shown_level(&harness).ok_or("no mip shown")?;
    let side = 1024_f32 / 2_f32.powi(i32::try_from(level)?);
    let facts = harness
        .get_by_label_contains("Format B8G8R8A8_UNORM")
        .rect();
    let selector = harness.get_by_label("1024").rect();
    let status = harness.get_by_label("10 Entries").rect();
    let width = 2.0f32.mul_add(-PREVIEW_PADDING, facts.width());
    let height = status.top() - selector.bottom() - SELECTOR_GAP - PREVIEW_PADDING_Y;
    assert!(
        harness
            .query_by_role_and_label(Role::Image, &format!("Mip {level}: {side} × {side} pixels"))
            .is_some(),
        "mip {level} is shown"
    );
    assert!(
        side <= width && side <= height,
        "{side} fits into {width} × {height}"
    );
    assert!(
        2.0 * side > width.min(height),
        "the next larger mip would not fit into {width} × {height}"
    );
    Ok(())
}

#[test]
fn the_mip_selector_switches_levels() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "panel_diff.dds")?;

    harness.get_by_label("1024").click();
    wait_until(&mut harness, |model| {
        settled(model) && image(model).and_then(ImagePreview::level) == Some(0)
    })?;

    assert_eq!(shown_level(&harness), Some(0), "largest mip");
    assert!(
        harness
            .query_by_label("Mip 0: 1024 × 1024 pixels")
            .is_some(),
        "shown"
    );

    harness.get_by_label("2").click();
    wait_until(&mut harness, |model| {
        settled(model) && image(model).and_then(ImagePreview::level) == Some(9)
    })?;

    assert!(
        harness.query_by_label("Mip 9: 2 × 2 pixels").is_some(),
        "a small mip from the header"
    );
    Ok(())
}

#[test]
fn the_info_shows_format_size_mips_and_cubemap_faces() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "panel_diff.dds")?;
    assert!(
        harness
            .query_by_label("Format B8G8R8A8_UNORM Size 1024 × 1024 Mips 11 Cubemap faces –")
            .is_some(),
        "plain texture"
    );

    select(&mut harness, "probe_cm.dds")?;
    assert!(
        harness
            .query_by_label("Format B8G8R8A8_UNORM Size 16 × 16 Mips 5 Cubemap faces 6")
            .is_some(),
        "cubemap"
    );
    assert!(
        harness.query_by_label("Mip 0: 64 × 48 pixels").is_some(),
        "the faces are shown as a cross"
    );
    Ok(())
}

#[test]
fn an_unsupported_format_is_named_and_the_text_and_hex_tabs_still_work() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "odd_spec.dds")?;

    assert!(
        harness
            .query_by_label(
                "The preview cannot decode R8G8B8A8_SINT textures. The Text and Hex tabs \
                 still show the data of Data\\Textures\\odd_spec.dds."
            )
            .is_some(),
        "message"
    );
    harness.get_by_label("Hex").click();
    harness.run_quiet();
    assert!(
        harness
            .query_by_label_contains("00000000  44 44 53 20")
            .is_some(),
        "hex shows the DDS magic"
    );
    harness.get_by_label("Text").click();
    harness.run_quiet();
    assert_eq!(harness.state().preview_tab(), PreviewTab::Text, "Text tab");
    Ok(())
}

#[test]
fn a_mip_that_cannot_be_read_names_the_reason() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "lost_ddna.dds")?;

    assert!(
        harness
            .query_by_label_contains("Cannot decode Data\\Textures\\lost_ddna.dds: ")
            .is_some(),
        "failure"
    );
    Ok(())
}

#[test]
fn selecting_another_entry_leaves_the_image_tab() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "panel_diff.dds")?;

    harness.get_by_label("readme.txt").click();
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Text, "Text tab");
    assert!(harness.query_by_label("plain text").is_some(), "text shown");
    assert!(harness.query_by_label("Image").is_none(), "no Image tab");
    Ok(())
}

#[test]
fn decoding_runs_off_the_window_thread() -> TestResult {
    let (folder, path) = archive(textures())?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    wait_polling(
        &mut model,
        "the Archive to open",
        |model| {
            model.poll();
        },
        |model| model.browser().is_some(),
    )?;
    let browser = model.browser_mut().ok_or("not opened")?;
    let id = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == "panel_diff.dds")
        .ok_or("no texture")?;

    browser.select_entry(id, || {});
    let opening = browser.image_mut().map(|image| image.is_busy());
    let still_opening = Cell::new(true);
    wait_polling(
        browser,
        "the header to be read",
        |browser| {
            browser.poll();
            still_opening.set(
                browser
                    .image_mut()
                    .is_some_and(|image| matches!(image.status(), ImageStatus::Opening)),
            );
        },
        |_| !still_opening.get(),
    )?;
    let image = browser.image_mut().ok_or("no image preview")?;
    image.request(0, 2048);
    let decoding = matches!(
        image.status(),
        ImageStatus::Opened(_, MipStatus::Decoding(0))
    );

    assert_eq!(opening, Some(true), "the header is read in the background");
    assert!(decoding, "the request returns while the mip is decoded");
    drop(folder);
    Ok(())
}

#[test]
fn the_german_image_tab_uses_german_texts() -> TestResult {
    let (_folder, mut harness) = opened(Language::German)?;

    select(&mut harness, "panel_diff.dds")?;

    assert!(harness.query_by_label("Bild").is_some(), "tab");
    assert!(
        harness
            .query_by_label("Format B8G8R8A8_UNORM Größe 1024 × 1024 Mips 11 Cubemap-Seiten –")
            .is_some(),
        "facts"
    );
    Ok(())
}

#[test]
fn the_focused_picture_scrolls_with_the_keyboard() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "panel_diff.dds")?;
    harness.get_by_label("1024").click();
    wait_until(&mut harness, |model| {
        settled(model) && image(model).and_then(ImagePreview::level) == Some(0)
    })?;
    let top = |harness: &egui_kittest::Harness<'_, Model>| {
        harness
            .get_by_role_and_label(Role::Image, "Mip 0: 1024 × 1024 pixels")
            .rect()
            .top()
    };
    let at_start = top(&harness);

    harness.get_by_label("Picture").focus();
    harness.run_quiet();
    harness.key_press(Key::ArrowDown);
    harness.run_quiet();
    let one_row = top(&harness);
    harness.key_press(Key::PageDown);
    harness.run_quiet();
    let a_page = top(&harness);
    harness.key_press(Key::End);
    harness.run_quiet();
    let at_end = top(&harness);
    harness.key_press(Key::Home);
    harness.run_quiet();

    assert!(
        one_row < at_start,
        "an arrow key scrolls: {at_start} {one_row}"
    );
    assert!(
        a_page < one_row,
        "a page scrolls further: {one_row} {a_page}"
    );
    assert!(at_end < a_page, "End scrolls to the end: {a_page} {at_end}");
    assert!(
        (top(&harness) - at_start).abs() < 0.5,
        "Home returns to the start"
    );
    Ok(())
}
