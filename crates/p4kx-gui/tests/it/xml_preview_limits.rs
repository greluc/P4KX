// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The limits, Archive changes and keyboard scrolling of the XML tab, driven headless.

use crate::xml_previewing;

use egui::Key;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, PreviewStatus, PreviewTab, XML_TEXT_LIMIT, XmlPreview, XmlStatus};
use p4kx_testkit::TestEntry;
use xml_previewing::{TestResult, archive, entries, opened, opened_at, select, wait_until, xml};
#[test]
fn an_entry_too_large_to_convert_says_so_in_the_xml_tab() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "huge.xml")?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Xml, "XML tab");
    assert!(
        harness
            .query_by_label_contains("are not converted in the preview")
            .is_some(),
        "too large message"
    );
    Ok(())
}

#[test]
fn xml_past_the_limit_is_cut_and_marked_as_truncated() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "long.xml")?;

    let (truncated, size) = match xml(harness.state()).map(XmlPreview::status) {
        Some(XmlStatus::Ready(document)) => (document.is_truncated(), document.bytes().len()),
        other => return Err(format!("not converted: {other:?}").into()),
    };
    assert!(truncated, "truncated");
    assert!(size <= XML_TEXT_LIMIT, "{size} bytes kept");
    assert!(
        harness
            .query_by_label_contains("Truncated: showing the first ")
            .is_some(),
        "truncation note"
    );
    Ok(())
}

#[test]
fn a_cryxml_entry_of_a_changed_archive_is_not_converted() -> TestResult {
    let (folder, path) = archive()?;
    let mut harness = opened_at(Language::BritishEnglish, path.clone())?;
    select(&mut harness, "ship.xml")?;

    entries()
        .entry(TestEntry::stored("Data\\patch.txt", "patch"))
        .write_to(&path)?;
    let browser = harness.state_mut().browser_mut().ok_or("not opened")?;
    let fleet = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == "fleet.xml")
        .ok_or("no fleet")?;
    browser.select_entry(fleet, || {});
    wait_until(&mut harness, |model| {
        model.browser().is_some_and(|browser| {
            browser.preview().is_some_and(|preview| {
                browser.index().name(preview.entry()) == "fleet.xml"
                    && matches!(preview.status(), PreviewStatus::ArchiveChanged)
            })
        })
    })?;

    assert!(xml(harness.state()).is_none(), "no XML preview");
    assert!(harness.query_by_label("XML").is_none(), "no XML tab");
    assert!(
        harness
            .query_by_label("The Archive changed on disk. Reload it to preview Entries.")
            .is_some(),
        "archive change message"
    );
    drop(folder);
    Ok(())
}

#[test]
fn the_focused_xml_view_scrolls_with_the_keyboard() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "fleet.xml")?;
    let first = "  <ship name=\"ship-0\"/>";
    let declaration = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

    harness.get_by_label("Converted XML").focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    assert!(
        harness.query_by_label("</fleet>").is_some(),
        "End shows the last line"
    );
    assert!(
        harness.query_by_label(first).is_none(),
        "the first ship scrolled away"
    );

    harness.key_press(Key::Home);
    harness.run();
    assert!(
        harness.query_by_label(first).is_some(),
        "Home shows the first ship"
    );

    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(
        harness.query_by_label(declaration).is_none(),
        "one line down"
    );
    assert!(harness.query_by_label(first).is_some(), "only one line");

    harness.key_press(Key::PageDown);
    harness.run();
    assert!(harness.query_by_label(first).is_none(), "a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!(
        harness.query_by_label(declaration).is_some(),
        "back at the top"
    );
    Ok(())
}
