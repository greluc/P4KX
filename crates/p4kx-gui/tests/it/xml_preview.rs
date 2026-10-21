//! The XML tab of the preview over a synthetic Archive with `CryXml` Entries, driven headless.

use crate::xml_previewing;

use std::time::{Duration, Instant};

use egui::OutputCommand;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, Preview, PreviewTab, XmlPreview, XmlStatus};
use xml_previewing::{TestResult, archive, opened, select, xml};

const SHIP_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ship name=\"Aurora\">\n  \
                        <part name=\"hull\"/>\n  <part name=\"engine\">thrust</part>\n</ship>\n";
#[test]
fn selecting_a_cryxml_entry_shows_its_converted_xml_in_the_xml_tab() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "ship.xml")?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Xml, "XML tab");
    let text = match xml(harness.state()).map(XmlPreview::status) {
        Some(XmlStatus::Ready(text)) => text.bytes().to_vec(),
        other => return Err(format!("not converted: {other:?}").into()),
    };
    assert_eq!(String::from_utf8(text)?, SHIP_XML, "converted XML");
    assert!(harness.query_by_label("XML").is_some(), "XML tab offered");
    for line in [
        "<ship name=\"Aurora\">",
        "  <part name=\"hull\"/>",
        "  <part name=\"engine\">thrust</part>",
    ] {
        assert!(
            harness.query_by_label(line).is_some(),
            "line {line:?} shown"
        );
    }
    Ok(())
}

#[test]
fn the_xml_tab_is_not_offered_for_entries_that_are_not_cryxml() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "ship.xml")?;

    for name in ["plain.xml", "readme.txt"] {
        select(&mut harness, name)?;

        assert!(xml(harness.state()).is_none(), "{name} is not CryXml");
        assert!(
            harness.query_by_label("XML").is_none(),
            "no XML tab for {name}"
        );
        assert_eq!(
            harness.state().preview_tab(),
            PreviewTab::Text,
            "{name} in Text"
        );
    }
    Ok(())
}

#[test]
fn an_entry_that_fails_to_convert_shows_the_error_and_text_and_hex_still_work() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "broken.xml")?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Xml, "XML tab");
    assert!(
        harness
            .query_by_label_contains("Cannot convert Data\\Libs\\broken.xml to XML: ")
            .is_some(),
        "error in the tab"
    );
    harness.get_by_label("Hex").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("00000000  43 72 79 58 6D 6C 42 00")
            .is_some(),
        "hex shows the signature"
    );
    harness.get_by_label("Text").click();
    harness.run();
    assert_eq!(harness.state().preview_tab(), PreviewTab::Text, "Text tab");
    assert!(
        harness.query_by_label_contains("CryXmlB").is_some(),
        "text shows the data"
    );
    Ok(())
}

#[test]
fn a_large_entry_is_converted_off_the_window_thread() -> TestResult {
    let (folder, path) = archive()?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    let browser = model.browser_mut().ok_or("not opened")?;
    let id = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == "fleet.xml")
        .ok_or("no fleet")?;

    let started = Instant::now();
    browser.select_entry(id, || {});
    let returned = started.elapsed();
    let loading = browser.preview().is_some_and(Preview::is_loading);
    let deadline = Instant::now() + Duration::from_secs(20);
    let lines = loop {
        browser.poll();
        if let Some(XmlStatus::Ready(text)) = browser
            .preview()
            .and_then(Preview::xml)
            .map(XmlPreview::status)
        {
            break text.line_count();
        }
        if Instant::now() > deadline {
            return Err("not converted in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };

    assert!(loading, "the Entry is read and converted in the background");
    assert!(
        returned < Duration::from_millis(100),
        "selecting returned in {returned:?}"
    );
    assert_eq!(lines, 50_003, "declaration, root, 50,000 ships, end tag");
    drop(folder);
    Ok(())
}

#[test]
fn only_the_visible_lines_of_a_large_entry_are_shown() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "fleet.xml")?;

    let shown = harness
        .query_all_by_label_contains("<ship name=\"ship-")
        .count();
    assert!(
        harness
            .query_by_label("  <ship name=\"ship-0\"/>")
            .is_some(),
        "the first ship is shown"
    );
    assert!((1..64).contains(&shown), "{shown} of 50,000 lines shown");
    Ok(())
}

#[test]
fn copy_puts_the_xml_on_the_clipboard() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "ship.xml")?;

    harness.get_by_label("Copy").click();
    harness.step();

    let copied = harness
        .output()
        .platform_output
        .commands
        .iter()
        .any(|command| matches!(command, OutputCommand::CopyText(text) if text == SHIP_XML));
    assert!(copied, "the XML is copied");
    Ok(())
}

#[test]
fn the_german_xml_tab_uses_german_texts() -> TestResult {
    let (_folder, mut harness) = opened(Language::German)?;

    select(&mut harness, "broken.xml")?;

    assert!(harness.query_by_label("XML").is_some(), "tab");
    assert!(
        harness
            .query_by_label_contains(
                "Data\\Libs\\broken.xml kann nicht in XML umgewandelt werden: "
            )
            .is_some(),
        "error"
    );
    Ok(())
}
