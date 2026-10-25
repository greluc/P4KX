// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the XML preview tests: a synthetic Archive with `CryXml` Entries opened in a
//! headless window.

use crate::waiting::{RunQuiet, wait_for_model};
use std::error::Error;
use std::path::PathBuf;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    Browser, Language, Model, PREVIEW_READ_LIMIT, Preview, XML_SIZE_LIMIT, XML_TEXT_LIMIT,
    XmlPreview, XmlStatus, ui,
};
use p4kx_testkit::{ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry};
use tempfile::TempDir;
pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) fn ship() -> Vec<u8> {
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

pub(crate) fn fleet(ships: usize) -> Vec<u8> {
    (0..ships)
        .fold(TestElement::new("fleet"), |fleet, index| {
            fleet.child(TestElement::new("ship").attribute("name", format!("ship-{index}")))
        })
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

pub(crate) fn long() -> Vec<u8> {
    let value = "v".repeat(64 * 1024);
    let children = XML_TEXT_LIMIT / value.len() + 16;
    (0..children)
        .fold(TestElement::new("long"), |long, _| {
            long.child(TestElement::new("item").attribute("value", value.as_str()))
        })
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

pub(crate) fn huge_start() -> Vec<u8> {
    let mut start = ship();
    start.resize(PREVIEW_READ_LIMIT + 1, 0);
    start
}

pub(crate) fn entries() -> ArchiveBuilder {
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

pub(crate) fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    entries().write_to(&path)?;
    Ok((folder, path))
}

pub(crate) fn opened(
    language: Language,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (folder, path) = archive()?;
    Ok((folder, opened_at(language, path)?))
}

pub(crate) fn opened_at(
    language: Language,
    path: PathBuf,
) -> Result<Harness<'static, Model>, Box<dyn Error>> {
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

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    wait_for_model(harness, "the window to settle", done)?;
    harness.run_quiet();
    Ok(())
}

pub(crate) fn xml(model: &Model) -> Option<&XmlPreview> {
    model
        .browser()
        .and_then(Browser::preview)
        .and_then(Preview::xml)
}

pub(crate) fn settled(model: &Model) -> bool {
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

pub(crate) fn select(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, settled)
}
