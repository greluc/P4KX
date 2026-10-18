//! The Comparison tab over two real Archives; runs with `--ignored` when `P4KX_DATA_P4K` and
//! `P4KX_DATA_P4K_B` name them.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{
    ChangeColumn, ComparisonLevels, ComparisonSide, ComparisonStatus, ComparisonTab, Language,
    Model, ReportStatus, WorkTab, ui,
};
use p4kx_testkit::{ArchiveBuilder, real_archive_b_path, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const RUN_BUDGET: Duration = Duration::from_secs(60);
const VIEW_BUDGET: Duration = Duration::from_millis(500);
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

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn two_real_archives_compare_and_the_list_stays_quick_to_sort_and_filter() -> TestResult {
    let (old, new) = (real_archive_path()?, real_archive_b_path()?);
    let mut model = Model::new(Language::BritishEnglish);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old);
    tab.set_archive(ComparisonSide::New, new);

    assert!(tab.start(|| {}), "started");
    wait(tab, |tab| !tab.is_running())?;
    let view = tab.view_mut().ok_or("no result")?;
    let sorted_at = Instant::now();
    view.sort_by(ChangeColumn::NewSize);
    view.sort_by(ChangeColumn::NewSize);
    let sorting = sorted_at.elapsed();
    let total = view.summary().counts.total();
    view.set_filter("*.xml", || {});
    wait(tab, |tab| {
        tab.view().is_some_and(|view| !view.is_filtering())
    })?;

    let view = tab.view().ok_or("no result")?;
    assert_eq!(
        view.comparison().len(),
        total,
        "every change is listed before filtering"
    );
    assert!(sorting < VIEW_BUDGET, "sorting took {sorting:?}");
    assert!(
        view.summary().counts.total() <= total,
        "the filter narrows the counts"
    );
    assert!(
        matches!(tab.status(), ComparisonStatus::Finished(_)),
        "finished"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_comparison_cancels_promptly() -> TestResult {
    let path = real_archive_path()?;
    let mut model = Model::new(Language::BritishEnglish);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, path.clone());
    tab.set_archive(ComparisonSide::New, path);

    assert!(tab.start(|| {}), "started");
    std::thread::sleep(Duration::from_millis(200));
    let cancelled_at = Instant::now();
    tab.cancel();

    assert!(
        cancelled_at.elapsed() < Duration::from_millis(50),
        "the tab stops at once"
    );
    assert!(
        matches!(tab.status(), ComparisonStatus::Cancelled),
        "cancelled"
    );
    let deadline = Instant::now() + RUN_BUDGET;
    while tab.is_stopping() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!tab.is_stopping(), "the background thread ended");
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

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn two_real_archives_compare_on_both_levels_and_save_the_report() -> TestResult {
    let out = tempfile::tempdir()?;
    let report = out.path().join("report.json");
    let mut model = Model::new(Language::BritishEnglish);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, real_archive_path()?);
    tab.set_archive(ComparisonSide::New, real_archive_b_path()?);
    tab.set_levels(ComparisonLevels {
        entries: true,
        records: true,
    });

    assert!(tab.start(|| {}), "started");
    wait(tab, |tab| !tab.is_running())?;
    if tab.view().is_none() {
        return Err(format!("no result: {:?}", tab.status()).into());
    }
    tab.view_mut()
        .ok_or("no result")?
        .save_report(report.clone(), || {});
    wait(tab, |tab| tab.view().is_some_and(|view| !view.is_saving()))?;

    let saved = tab.view().and_then(|view| view.report_status());
    assert!(
        matches!(saved, Some(ReportStatus::Saved(_))),
        "saved: {saved:?}"
    );
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&report)?)?;
    assert_eq!(
        json["levels"],
        serde_json::json!(["entries", "records"]),
        "both levels"
    );
    Ok(())
}
