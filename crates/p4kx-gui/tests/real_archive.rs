//! Browsing a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.
//!
//! The time limits assume a release build.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_extraction::Archive;
use p4kx_gui::{EntryIndex, Language, Model, Sort, SortColumn, SortDirection, ui};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const SORT_LIMIT: Duration = Duration::from_millis(30);
const FRAME_LIMIT: Duration = Duration::from_millis(16);
const COLUMNS: [SortColumn; 4] = [
    SortColumn::Name,
    SortColumn::Compression,
    SortColumn::Encryption,
    SortColumn::Size,
];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn sorting_every_entry_of_a_real_archive_takes_milliseconds() -> TestResult {
    let index = EntryIndex::build(Archive::open(real_archive_path()?)?);

    for column in COLUMNS {
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            let started = Instant::now();
            let shown = index.entries_in(index.root(), Sort { column, direction });
            let took = started.elapsed();

            assert_eq!(shown.len(), index.len(), "every Entry is shown");
            assert!(took < SORT_LIMIT, "{column:?} {direction:?} took {took:?}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn frames_stay_short_while_sorting_and_scrolling_a_real_archive() -> TestResult {
    let mut harness = Harness::builder()
        .with_size([1400.0, 900.0])
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
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
    assert!(
        harness
            .state()
            .browser()
            .is_some_and(|browser| browser.shown().len() > 100_000),
        "a large Archive"
    );

    for header in ["Size", "Size", "Compression", "Encryption", "Name"] {
        harness.get_by_label(header).click();
        let took = timed_step(&mut harness);
        assert!(took < FRAME_LIMIT, "sorting by {header} took {took:?}");
    }
    harness.hover_at(egui::pos2(800.0, 450.0));
    let first_row = top_row_name(&harness);
    for _ in 0..20 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, -500.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
            source: egui::MouseWheelSource::Wheel,
        });
        let took = timed_step(&mut harness);
        assert!(took < FRAME_LIMIT, "scrolling took {took:?}");
    }
    harness.run_steps(2);
    assert_ne!(top_row_name(&harness), first_row, "the table scrolled");
    Ok(())
}

fn top_row_name(harness: &Harness<'_, Model>) -> Option<String> {
    harness
        .get_all_by_label_contains("Select ")
        .filter_map(|node| node.accesskit_node().label())
        .find(|label| !label.starts_with("Select all"))
}

fn timed_step(harness: &mut Harness<'_, Model>) -> Duration {
    let started = Instant::now();
    harness.step();
    started.elapsed()
}
