//! The line diff of the Comparison tab over two real Archives; runs with `--ignored` when
//! `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B` name them.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_comparison::ChangeKind;
use p4kx_gui::{
    ComparisonSide, ComparisonTab, ComparisonView, DiffStatus, Language, Model, WorkTab, ui,
};
use p4kx_testkit::{real_archive_b_path, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const RUN_BUDGET: Duration = Duration::from_secs(120);
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

fn loaded(tab: &ComparisonTab) -> bool {
    tab.view()
        .and_then(ComparisonView::diff)
        .is_some_and(|diff| !matches!(diff.status(), DiffStatus::Loading))
}

/// Returns the changed XML Entries of the finished Comparison, largest new side first.
fn changed_xml(view: &ComparisonView) -> Vec<u32> {
    let mut ids: Vec<(u64, u32)> = view
        .shown()
        .iter()
        .filter_map(|&id| {
            let change = view.change(id)?;
            let size = change.new_entry()?.uncompressed_size();
            (change.kind() == ChangeKind::Changed
                && change.path().to_ascii_lowercase().ends_with(".xml"))
            .then_some((size, id))
        })
        .collect();
    ids.sort_unstable_by(|a, b| b.cmp(a));
    ids.into_iter().map(|(_, id)| id).collect()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_changed_real_xml_entry_diffs_off_the_ui_thread_and_frames_stay_short() -> TestResult {
    let mut model = Model::new(Language::BritishEnglish);
    model.set_work_tab(WorkTab::Comparison);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, real_archive_path()?);
    tab.set_archive(ComparisonSide::New, real_archive_b_path()?);
    assert!(tab.start(|| {}), "started");
    wait(tab, |tab| !tab.is_running())?;
    let view = tab.view_mut().ok_or("no result")?;
    let candidates = changed_xml(view);
    let (&largest, &other) = (
        candidates.first().ok_or("no changed XML Entry")?,
        candidates.get(1).ok_or("a second changed XML Entry")?,
    );

    view.select(largest);
    let started = Instant::now();
    view.load_diff(|| {});
    let handed_off = started.elapsed();
    view.select(other);
    view.load_diff(|| {});
    wait(tab, loaded)?;

    let diff = tab.view().and_then(ComparisonView::diff).ok_or("no diff")?;
    assert!(
        handed_off < Duration::from_millis(5),
        "starting the diff took {handed_off:?}"
    );
    assert_eq!(diff.change(), other, "the last selection wins");
    assert!(
        !matches!(diff.status(), DiffStatus::Failed(_)),
        "the diff failed: {:?}",
        diff.status()
    );
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1280.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run_steps(3);
    for _ in 0..20 {
        if let Some(next) = harness.query_by_label("Next change") {
            next.click();
        }
        let started = Instant::now();
        harness.step();
        let took = started.elapsed();
        assert!(took < FRAME_LIMIT, "a frame of the diff took {took:?}");
    }
    Ok(())
}
