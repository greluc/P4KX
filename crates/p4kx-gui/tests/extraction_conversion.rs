//! The Conversion switch of the Extraction setup, driven headless through `egui_kittest`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const SWITCH: &str = "Convert CryXml Entries to XML";

fn vehicle() -> Vec<u8> {
    TestElement::new("Vehicle")
        .attribute("name", "AEGS_Gladius")
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn opened(convert: bool) -> Result<(TempDir, PathBuf, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\vehicle.xml", vehicle()))
        .entry(TestEntry::stored("Data\\text.xml", b"<a/>".to_vec()))
        .write_to(&path)?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_extraction_folder(Some(out.clone()));
    model.set_extraction_conversion(convert);
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((dir, out, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    settle(harness);
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        settle(harness);
    }
    settle(harness);
    Ok(())
}

fn settle(harness: &mut Harness<'_, Model>) {
    harness.step();
    harness.step();
}

fn extract_shown(harness: &mut Harness<'_, Model>, switch: bool) -> TestResult {
    harness.get_by_label("Extract shown…").click();
    settle(harness);
    harness.get_by_label("Existing folder").click();
    settle(harness);
    if switch {
        harness.get_by_label(SWITCH).click();
        settle(harness);
    }
    harness.get_by_label("Extract").click();
    settle(harness);
    harness.get_by_label("Extract whole Archive").click();
    wait_until(harness, |model| {
        model
            .extraction()
            .is_some_and(|extraction| !extraction.is_running())
    })
}

fn read(out: &Path, path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(fs::read(out.join(path))?)
}

#[test]
fn the_switch_converts_cryxml_entries_and_the_summary_counts_them() -> TestResult {
    let (_dir, out, mut harness) = opened(false)?;

    extract_shown(&mut harness, true)?;

    harness.get_by_label("Extracted 2 (1 converted), skipped 0, failed 0, cancelled 0");
    assert!(
        read(&out, "Data/vehicle.xml")?.starts_with(b"<?xml"),
        "converted"
    );
    assert_eq!(read(&out, "Data/text.xml")?, b"<a/>", "text XML unchanged");
    assert!(harness.state().extraction_conversion(), "remembered");
    Ok(())
}

#[test]
fn the_switch_is_off_by_default_and_entries_stay_raw() -> TestResult {
    let (_dir, out, mut harness) = opened(false)?;

    extract_shown(&mut harness, false)?;

    harness.get_by_label("Extracted 2, skipped 0, failed 0, cancelled 0");
    assert_eq!(read(&out, "Data/vehicle.xml")?, vehicle(), "raw CryXml");
    assert!(!harness.state().extraction_conversion(), "still off");
    Ok(())
}

#[test]
fn the_setup_starts_with_the_switch_as_stored() -> TestResult {
    let (_dir, _out, mut harness) = opened(true)?;

    harness.get_by_label("Extract shown…").click();
    settle(&mut harness);

    assert!(
        harness
            .state()
            .extraction_setup()
            .is_some_and(p4kx_gui::ExtractionSetup::convert),
        "preset from the settings"
    );
    harness.get_by_label(SWITCH).click();
    settle(&mut harness);
    assert!(
        harness
            .state()
            .extraction_setup()
            .is_some_and(|setup| !setup.convert()),
        "switched off"
    );
    Ok(())
}
