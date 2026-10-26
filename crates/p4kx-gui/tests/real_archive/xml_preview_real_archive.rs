// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The XML preview over the `CryXml` Entries of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.
//!
//! The time limits assume a release build.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use p4kx_extraction::Archive;
use p4kx_gui::{
    Browser, EntryIndex, Language, Model, Preview, PreviewTab, XML_SIZE_LIMIT, XmlPreview,
    XmlStatus, ui,
};
use p4kx_testkit::{assert_steady_frames, real_archive_path};

use crate::frame_steps::step_into;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLES: usize = 300;
const WAIT: Duration = Duration::from_mins(1);
const FRAME_LIMIT: Duration = Duration::from_millis(50);
const FRAME_CEILING: Duration = Duration::from_millis(250);

fn sampled_xml(index: &EntryIndex) -> Result<Vec<u32>, Box<dyn Error>> {
    let candidates: Vec<u32> = (0..u32::try_from(index.len())?)
        .filter(|&id| {
            index.entry(id).is_some_and(|entry| {
                entry.path().to_ascii_lowercase().ends_with(".xml")
                    && entry.uncompressed_size() <= XML_SIZE_LIMIT
            })
        })
        .collect();
    let step = candidates.len().div_ceil(SAMPLES).max(1);
    Ok(candidates.into_iter().step_by(step).collect())
}

fn settled(browser: &Browser) -> bool {
    browser.preview().is_some_and(|preview| {
        !preview.is_loading()
            && preview
                .xml()
                .is_none_or(|xml| !matches!(xml.status(), XmlStatus::Converting))
    })
}

fn wait(browser: &mut Browser) -> TestResult {
    let deadline = Instant::now() + WAIT;
    while !settled(browser) {
        if Instant::now() > deadline {
            return Err("the preview did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(2));
        browser.poll();
    }
    Ok(())
}

fn xml(browser: &Browser) -> Option<&XmlPreview> {
    browser.preview().and_then(Preview::xml)
}

/// Converts the sampled Entries; returns the converted Entry with the most lines.
fn largest_converted() -> Result<(u32, usize), Box<dyn Error>> {
    let index = EntryIndex::build(Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::test_key()),
    )?);
    let samples = sampled_xml(&index)?;
    let mut browser = Browser::new(index);
    let mut largest: Option<(u32, usize)> = None;
    let mut failed = Vec::new();
    for id in samples {
        browser.select_entry(id, || {});
        wait(&mut browser)?;
        match xml(&browser).map(XmlPreview::status) {
            Some(XmlStatus::Ready(document)) => {
                assert!(document.line_count() > 0, "Entry {id} has lines");
                if largest.is_none_or(|(_, lines)| document.line_count() > lines) {
                    largest = Some((id, document.line_count()));
                }
            }
            Some(XmlStatus::Failed(reason)) => failed.push((id, reason.map(str::to_owned))),
            _ => {}
        }
    }
    assert!(
        failed.is_empty(),
        "every sampled CryXml Entry converts: {failed:?}"
    );
    largest.ok_or_else(|| "no sampled Entry is CryXml".into())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_cryxml_entries_of_a_real_archive_convert_and_frames_stay_short() -> TestResult {
    let (id, lines) = largest_converted()?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    model.open(real_archive_path()?, || {});
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    let deadline = Instant::now() + WAIT;
    while harness.state().browser().is_none() {
        assert!(
            Instant::now() < deadline,
            "the Archive did not open in time"
        );
        std::thread::sleep(Duration::from_millis(20));
        harness.step();
    }
    harness
        .state_mut()
        .browser_mut()
        .ok_or("not opened")?
        .select_entry(id, || {});

    let mut frames = Vec::new();
    let deadline = Instant::now() + WAIT;
    while !harness.state().browser().is_some_and(settled) {
        assert!(
            Instant::now() < deadline,
            "converting did not finish in time"
        );
        step_into(&mut frames, &mut harness);
        std::thread::sleep(Duration::from_millis(5));
    }
    for _ in 0..3 {
        step_into(&mut frames, &mut harness);
    }

    assert_eq!(harness.state().preview_tab(), PreviewTab::Xml, "XML tab");
    assert_steady_frames(
        &format!("converting {lines} lines"),
        &frames,
        FRAME_LIMIT,
        FRAME_CEILING,
    );
    Ok(())
}
