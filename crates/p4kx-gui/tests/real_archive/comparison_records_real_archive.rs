//! The Record list of the Comparison tab over two real Archives; runs with `--ignored` when
//! `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B` name them, and skips without the second.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use p4kx_comparison::RecordChangeKind;
use p4kx_gui::{
    ComparisonLevels, ComparisonList, ComparisonSide, ComparisonStatus, ComparisonTab,
    ComparisonView, Language, Model, RecordChangeColumn, RecordChangeView, RecordDetailStatus,
    WorkTab, ui,
};
use p4kx_testkit::real_archive_pair;

type TestResult = Result<(), Box<dyn Error>>;

const RUN_BUDGET: Duration = Duration::from_secs(600);
const VIEW_BUDGET: Duration = Duration::from_millis(500);
const DETAIL_BUDGET: Duration = Duration::from_secs(10);
const FRAME_LIMIT: Duration = Duration::from_millis(16);

fn wait(
    tab: &mut ComparisonTab,
    budget: Duration,
    done: impl Fn(&ComparisonTab) -> bool,
) -> TestResult {
    let deadline = Instant::now() + budget;
    while !done(tab) {
        if Instant::now() > deadline {
            return Err(format!("the Comparison did not settle: {:?}", tab.status()).into());
        }
        std::thread::sleep(Duration::from_millis(20));
        tab.poll();
    }
    Ok(())
}

fn records(tab: &ComparisonTab) -> Option<&RecordChangeView> {
    tab.view().and_then(ComparisonView::records)
}

fn compared_records() -> Result<Option<Model>, Box<dyn Error>> {
    let Some((old, new)) = real_archive_pair()? else {
        return Ok(None);
    };
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    model.set_work_tab(WorkTab::Comparison);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old);
    tab.set_archive(ComparisonSide::New, new);
    tab.set_levels(ComparisonLevels {
        entries: false,
        records: true,
    });
    if !tab.start(|| {}) {
        return Err("not started".into());
    }
    wait(tab, RUN_BUDGET, |tab| !tab.is_running())?;
    Ok(Some(model))
}

fn steady_frame(model: Model) -> Duration {
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1280.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run();
    let started = Instant::now();
    harness.step();
    started.elapsed()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn two_real_dataforges_list_their_record_changes_and_open_one() -> TestResult {
    let Some(mut model) = compared_records()? else {
        return Ok(());
    };
    let tab = model.comparison_mut();
    assert!(
        matches!(tab.status(), ComparisonStatus::Finished(_)),
        "finished: {:?}",
        tab.status()
    );
    let view = tab.view_mut().ok_or("no result")?;
    assert_eq!(
        view.list(),
        ComparisonList::Records,
        "the Record list first"
    );
    let list = view.records_mut().ok_or("no Record list")?;
    let total = list.shown().len();
    let sorted_at = Instant::now();
    list.sort_by(RecordChangeColumn::Type);
    list.sort_by(RecordChangeColumn::Change);
    let sorting = sorted_at.elapsed();
    let changed = list.shown().iter().copied().find(|&id| {
        list.row(id)
            .is_some_and(|row| row.kind() == RecordChangeKind::Changed)
    });
    if let Some(id) = changed {
        list.select(id);
    }
    view.load_record_detail(|| {});
    let opened_at = Instant::now();
    wait(tab, DETAIL_BUDGET, |tab| {
        changed.is_none()
            || records(tab)
                .and_then(RecordChangeView::detail)
                .is_some_and(|detail| !matches!(detail.status(), RecordDetailStatus::Loading))
    })?;
    let opening = opened_at.elapsed();

    let list = records(tab).ok_or("no Record list")?;
    assert_eq!(list.list().len(), total, "every Record change is listed");
    assert!(sorting < VIEW_BUDGET, "sorting took {sorting:?}");
    if changed.is_some() {
        let status = list.detail().map(|detail| detail.status());
        assert!(
            matches!(status, Some(RecordDetailStatus::Ready(fields)) if !fields.rows().is_empty()),
            "the changed Record has field changes: {status:?}"
        );
        assert!(opening < DETAIL_BUDGET, "opening took {opening:?}");
    }
    let frame = steady_frame(model);
    assert!(frame < FRAME_LIMIT * 4, "a steady frame took {frame:?}");
    Ok(())
}
