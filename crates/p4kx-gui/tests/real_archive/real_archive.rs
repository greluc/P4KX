// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Browsing a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.
//!
//! The time limits assume a release build.

use std::collections::BTreeSet;
use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_extraction::{Archive, Entry, Filter};
use p4kx_gui::{
    Browser, EntryIndex, ExtractionScope, Language, Model, RecordTree, RecordTreeStatus, Sort,
    SortColumn, SortDirection, ui,
};
use p4kx_testkit::{assert_steady_frames, real_archive_path};

use crate::frame_steps::step_into;
use crate::node_text::node_text;

type TestResult = Result<(), Box<dyn Error>>;

const SORT_LIMIT: Duration = Duration::from_millis(30);
const FRAME_LIMIT: Duration = Duration::from_millis(16);
const SORT_HEADERS: [&str; 12] = [
    "Size",
    "Size",
    "Compression",
    "Compression",
    "Encryption",
    "Encryption",
    "Name",
    "Name",
    "Size",
    "Compression",
    "Encryption",
    "Name",
];
const FRAME_CEILING: Duration = Duration::from_millis(100);
const COLUMNS: [SortColumn; 4] = [
    SortColumn::Name,
    SortColumn::Compression,
    SortColumn::Encryption,
    SortColumn::Size,
];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn sorting_every_entry_of_a_real_archive_takes_milliseconds() -> TestResult {
    let index = EntryIndex::build(Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?);

    for column in COLUMNS {
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            let started = Instant::now();
            let shown = index.entries_in(index.root(), Sort { column, direction });
            let took = started.elapsed();

            assert_eq!(
                shown.len() + index.folded_len(),
                index.len(),
                "every Entry is shown, the parts of a texture in its row"
            );
            assert!(took < SORT_LIMIT, "{column:?} {direction:?} took {took:?}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn split_textures_of_a_real_archive_are_grouped_once_and_stay_consistent() -> TestResult {
    let started = Instant::now();
    let groups = p4kx_extraction::TextureGroups::build(&Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?);
    let took = started.elapsed();
    assert!(!groups.is_empty(), "a real Archive holds split textures");
    assert!(took < Duration::from_secs(5), "grouping took {took:?}");

    let index = EntryIndex::build(Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?);
    assert!(index.folded_len() > 0, "parts are folded");
    let mut browser = Browser::new(index);
    let header = (0..u32::try_from(browser.index().len())?)
        .find(|&id| !browser.index().texture_parts(id).is_empty())
        .ok_or("no texture header")?;
    let parts = browser.index().texture_parts(header).len();
    browser.set_checked(header, true);
    assert_eq!(browser.checked_count(), parts + 1, "all parts checked");
    browser.toggle_texture(header);
    assert!(
        browser
            .rows()
            .iter()
            .any(|row| row.kind == p4kx_gui::EntryRowKind::Part),
        "the parts are reachable"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn frames_stay_short_while_sorting_and_scrolling_a_real_archive() -> TestResult {
    let mut harness = opened(false)?;
    assert!(
        harness
            .state()
            .browser()
            .is_some_and(|browser| browser.shown().len() > 100_000),
        "a large Archive"
    );

    let mut sorting = Vec::new();
    for header in SORT_HEADERS {
        harness.get_by_label(header).click();
        step_into(&mut sorting, &mut harness);
    }
    harness.hover_at(egui::pos2(800.0, 450.0));
    let first_row = top_row_name(&harness)?;
    let mut frames = Vec::new();
    for _ in 0..20 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, -500.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
            source: egui::MouseWheelSource::Wheel,
        });
        step_into(&mut frames, &mut harness);
    }
    harness.run_steps(2);
    assert_ne!(top_row_name(&harness)?, first_row, "the table scrolled");
    assert_steady_frames("sorting", &sorting, FRAME_LIMIT, FRAME_CEILING);
    assert_steady_frames("scrolling", &frames, FRAME_LIMIT, FRAME_CEILING);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn searching_a_real_archive_finds_what_p4kx_search_finds() -> TestResult {
    let path = real_archive_path()?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::real_archive_key()?))?;
    let groups = p4kx_extraction::TextureGroups::build(&archive);
    let mut browser = Browser::new(EntryIndex::build(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::real_archive_key()?),
    )?));

    for text in [
        "*.xml",
        "*.dds",
        "*.dcb",
        "Game2.dcb",
        "aurora",
        "Data/Libs/*.xml",
    ] {
        browser.search(text, || {});
        while browser.is_searching() {
            std::thread::sleep(Duration::from_millis(5));
            browser.poll();
        }
        let selected = Filter::new(text).select_with_parts(&archive, &groups);
        let expected: BTreeSet<&str> = selected.iter().map(|entry| entry.path()).collect();
        let covered = browser.scope_entries(ExtractionScope::Shown);
        let shown: BTreeSet<&str> = covered
            .iter()
            .filter_map(|&id| browser.index().entry(id).map(Entry::path))
            .collect();

        assert_eq!(shown, expected, "{text:?} matches as in p4kx extract");
        assert_eq!(
            covered.len(),
            selected.len(),
            "{text:?} covers each match once"
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn frames_stay_short_while_searching_a_real_archive() -> TestResult {
    let mut harness = opened(false)?;
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .click();
    harness.run_steps(2);

    let mut typing = Vec::new();
    for character in ["a", "u", "r", "o", "r", "a"] {
        harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .type_text(character);
        step_into(&mut typing, &mut harness);
    }
    let mut polling = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    while harness.state().browser().is_some_and(Browser::is_searching) {
        assert!(Instant::now() < deadline, "the search did not finish");
        std::thread::sleep(Duration::from_millis(2));
        step_into(&mut polling, &mut harness);
    }
    step_into(&mut polling, &mut harness);
    assert_steady_frames("typing", &typing, FRAME_LIMIT, FRAME_CEILING);
    assert_steady_frames("searching", &polling, FRAME_LIMIT, FRAME_CEILING);
    let shown = harness.state().browser().map_or(0, |b| b.shown().len());
    assert!(shown > 0, "aurora matches Entries");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn dataforge_of_a_real_archive_loads_while_frames_stay_short() -> TestResult {
    let mut harness = opened(true)?;

    let mut interactions = Vec::new();
    let mut polling = Vec::new();
    harness.get_by_label("Expand DataForge").click();
    step_into(&mut interactions, &mut harness);
    let deadline = Instant::now() + Duration::from_secs(120);
    while !records(&harness).is_some_and(|tree| {
        !matches!(
            tree.status(),
            RecordTreeStatus::Loading | RecordTreeStatus::NotLoaded
        )
    }) {
        assert!(Instant::now() < deadline, "DataForge did not load");
        std::thread::sleep(Duration::from_millis(2));
        step_into(&mut polling, &mut harness);
    }
    let Some(RecordTreeStatus::Ready(summary)) = records(&harness).map(RecordTree::status) else {
        return Err(format!("not loaded: {:?}", records(&harness)).into());
    };
    assert!(summary.record_count() > 50_000, "a real DataForge");
    for folder in ["libs", "foundry", "records"] {
        harness.get_by_label(&format!("Expand {folder}")).click();
        step_into(&mut interactions, &mut harness);
        step_into(&mut interactions, &mut harness);
    }
    let largest = records(&harness).and_then(|tree| {
        let folder = tree.find("libs/foundry/records")?;
        tree.node(folder)
            .children()
            .iter()
            .copied()
            .max_by_key(|&child| tree.node(child).children().len())
    });
    if let (Some(browser), Some(largest)) = (harness.state_mut().browser_mut(), largest) {
        browser.toggle_record_node(largest, || {});
    }
    step_into(&mut interactions, &mut harness);
    harness.hover_at(egui::pos2(150.0, 450.0));
    let mut scrolling = Vec::new();
    for _ in 0..20 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, -2.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
            source: egui::MouseWheelSource::Wheel,
        });
        step_into(&mut scrolling, &mut harness);
    }
    assert_steady_frames("expanding", &interactions, FRAME_LIMIT, FRAME_CEILING);
    assert_steady_frames("loading", &polling, FRAME_LIMIT, FRAME_CEILING);
    assert_steady_frames(
        "scrolling the Records",
        &scrolling,
        FRAME_LIMIT,
        FRAME_CEILING,
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn searching_the_records_of_a_real_archive_keeps_frames_short() -> TestResult {
    let mut harness = opened(true)?;
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "DataForge")
        .click();
    harness.run_steps(2);
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .click();
    harness.run_steps(2);

    let mut interactions = Vec::new();
    for character in ["s", "h", "i", "p", "s", "/"] {
        harness
            .get_by_role(egui::accesskit::Role::TextInput)
            .type_text(character);
        step_into(&mut interactions, &mut harness);
    }
    let mut polling = Vec::new();
    step_until_records_found(&mut harness, &mut polling);
    let browser = harness.state().browser().ok_or("no Archive")?;
    let tree = browser.records().ok_or("no DataForge")?;
    let matches = browser.record_matches().ok_or("no matches")?;
    let filter = Filter::new("ships/");
    assert_eq!(browser.record_search_text(), "ships/", "every key arrived");
    assert!(matches.len() > 100, "ships/ matches Records");
    assert!(
        matches
            .iter()
            .filter_map(|&id| tree.record(id))
            .all(|record| {
                filter.matches(record.record_path()) || filter.matches(record.type_name())
            }),
        "every match matches the Filter"
    );
    let first = *matches.first().ok_or("no first match")?;
    if let Some(browser) = harness.state_mut().browser_mut() {
        browser.pick_record(first);
    }
    step_into(&mut interactions, &mut harness);
    step_into(&mut interactions, &mut harness);
    assert_steady_frames(
        "typing and picking",
        &interactions,
        FRAME_LIMIT,
        FRAME_CEILING,
    );
    assert_steady_frames(
        "searching the Records",
        &polling,
        FRAME_LIMIT,
        FRAME_CEILING,
    );
    let tree = records(&harness).ok_or("no DataForge")?;
    assert_eq!(tree.selected(), tree.record_node(first), "picked");
    Ok(())
}

fn step_until_records_found(harness: &mut Harness<'_, Model>, frames: &mut Vec<Duration>) {
    let deadline = Instant::now() + Duration::from_secs(120);
    while harness
        .state()
        .browser()
        .is_some_and(|browser| browser.is_searching_records() || browser.record_matches().is_none())
    {
        assert!(
            Instant::now() < deadline,
            "the Record search did not finish"
        );
        std::thread::sleep(Duration::from_millis(2));
        step_into(frames, harness);
    }
    step_into(frames, harness);
}

fn records<'h>(harness: &'h Harness<'_, Model>) -> Option<&'h RecordTree> {
    harness.state().browser().and_then(Browser::records)
}

fn opened(with_key: bool) -> Result<Harness<'static, Model>, Box<dyn Error>> {
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
    if with_key {
        harness
            .state_mut()
            .set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    }
    harness.state_mut().open(real_archive_path()?, || {});
    let deadline = Instant::now() + Duration::from_secs(120);
    while harness.state().browser().is_none() {
        if Instant::now() > deadline {
            return Err(format!("not opened: {:?}", harness.state().failure()).into());
        }
        std::thread::sleep(Duration::from_millis(20));
        harness.step();
    }
    harness.run_steps(2);
    Ok(harness)
}

fn top_row_name(harness: &Harness<'_, Model>) -> Result<String, Box<dyn Error>> {
    harness
        .query_all_by_role(egui::accesskit::Role::CheckBox)
        .filter_map(|node| node_text(&node))
        .find(|label| label.starts_with("Select ") && !label.starts_with("Select all"))
        .ok_or_else(|| "no row of the table is shown".into())
}
