//! Frames of the Comparison tab over every Entry of a real Archive as a change; runs with
//! `--ignored` when `P4KX_DATA_P4K` names an Archive.
//!
//! This binary holds this one test only, so that no other test of the suite competes for the
//! cores while it measures frames.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{ComparisonSide, ComparisonTab, Language, Model, WorkTab, ui};
use p4kx_testkit::{ArchiveBuilder, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const RUN_BUDGET: Duration = Duration::from_secs(60);
const FRAME_LIMIT: Duration = Duration::from_millis(16);

fn wait(tab: &mut ComparisonTab, done: impl Fn(&ComparisonTab) -> bool) -> TestResult {
    let deadline = Instant::now() + RUN_BUDGET;
    while !done(tab) {
        if Instant::now() > deadline {
            return Err(format!("the Comparison did not settle: {:?}", tab.status()).into());
        }
        std::thread::sleep(Duration::from_millis(20));
        tab.poll();
    }
    Ok(())
}

fn timed_step(harness: &mut Harness<'_, Model>) -> Duration {
    let started = Instant::now();
    harness.step();
    started.elapsed()
}

fn top_row(harness: &Harness<'_, Model>) -> Option<String> {
    ["Added: ", "Removed: ", "Changed: "]
        .into_iter()
        .find_map(|kind| {
            harness
                .query_all_by_label_contains(kind)
                .find_map(|node| node.accesskit_node().label())
        })
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn frames_stay_short_while_sorting_and_scrolling_every_entry_as_a_change() -> TestResult {
    let empty = tempfile::tempdir()?;
    let new = empty.path().join("Data.p4k");
    ArchiveBuilder::new().write_to(&new)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_work_tab(WorkTab::Comparison);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, real_archive_path()?);
    tab.set_archive(ComparisonSide::New, new);
    assert!(tab.start(|| {}), "started");
    wait(tab, |tab| !tab.is_running())?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1280.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run_steps(2);

    for header in ["New size", "New size", "Old size", "Change", "Path"] {
        harness.get_by_label(header).click();
        let took = timed_step(&mut harness);
        assert!(took < FRAME_LIMIT, "sorting by {header} took {took:?}");
    }
    harness.hover_at(egui::pos2(800.0, 800.0));
    let first_row = top_row(&harness);
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
    let changes = harness
        .state()
        .comparison()
        .view()
        .map_or(0, |view| view.shown().len());
    assert!(
        changes > 100_000,
        "every Entry of a large Archive is removed"
    );
    assert_ne!(top_row(&harness), first_row, "the list scrolled");
    Ok(())
}
