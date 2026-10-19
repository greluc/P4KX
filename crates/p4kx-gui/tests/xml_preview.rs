//! The XML tab of the preview over a synthetic Archive with `CryXml` Entries, driven headless.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::Key;
use egui::OutputCommand;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    Browser, Language, Model, PREVIEW_READ_LIMIT, Preview, PreviewStatus, PreviewTab,
    XML_SIZE_LIMIT, XML_TEXT_LIMIT, XmlPreview, XmlStatus, ui,
};
use p4kx_testkit::{ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const SHIP_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ship name=\"Aurora\">\n  \
                        <part name=\"hull\"/>\n  <part name=\"engine\">thrust</part>\n</ship>\n";

fn ship() -> Vec<u8> {
    TestElement::new("ship")
        .attribute("name", "Aurora")
        .child(TestElement::new("part").attribute("name", "hull"))
        .child(
            TestElement::new("part")
                .attribute("name", "engine")
                .content("thrust"),
        )
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn fleet(ships: usize) -> Vec<u8> {
    (0..ships)
        .fold(TestElement::new("fleet"), |fleet, index| {
            fleet.child(TestElement::new("ship").attribute("name", format!("ship-{index}")))
        })
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn long() -> Vec<u8> {
    let value = "v".repeat(64 * 1024);
    let children = XML_TEXT_LIMIT / value.len() + 16;
    (0..children)
        .fold(TestElement::new("long"), |long, _| {
            long.child(TestElement::new("item").attribute("value", value.as_str()))
        })
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn huge_start() -> Vec<u8> {
    let mut start = ship();
    start.resize(PREVIEW_READ_LIMIT + 1, 0);
    start
}

fn entries() -> ArchiveBuilder {
    let mut broken = b"CryXmlB\0".to_vec();
    broken.extend_from_slice(&[0xFF; 24]);
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Libs\\ship.xml", ship()))
        .entry(TestEntry::stored("Data\\Libs\\broken.xml", broken))
        .entry(TestEntry::stored(
            "Data\\Libs\\plain.xml",
            "<config>\n  <ship name=\"Aurora\"/>\n</config>\n",
        ))
        .entry(TestEntry::zstd("Data\\Libs\\fleet.xml", fleet(50_000)))
        .entry(TestEntry::zstd("Data\\Libs\\long.xml", long()))
        .entry(
            TestEntry::stored("Data\\Libs\\huge.xml", huge_start())
                .with_uncompressed_size(XML_SIZE_LIMIT + 1),
        )
        .entry(TestEntry::stored("Data\\readme.txt", "plain text"))
}

fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    entries().write_to(&path)?;
    Ok((folder, path))
}

fn opened(language: Language) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (folder, path) = archive()?;
    Ok((folder, opened_at(language, path)?))
}

fn opened_at(language: Language, path: PathBuf) -> Result<Harness<'static, Model>, Box<dyn Error>> {
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok(harness)
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(20);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}

fn xml(model: &Model) -> Option<&XmlPreview> {
    model
        .browser()
        .and_then(Browser::preview)
        .and_then(Preview::xml)
}

fn settled(model: &Model) -> bool {
    model
        .browser()
        .and_then(Browser::preview)
        .is_some_and(|preview| {
            !preview.is_loading()
                && preview
                    .xml()
                    .is_none_or(|xml| !matches!(xml.status(), XmlStatus::Converting))
        })
}

fn select(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, settled)
}

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
