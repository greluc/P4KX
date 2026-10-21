//! The Comparison tab over two real Archives; runs with `--ignored` when `P4KX_DATA_P4K` and
//! `P4KX_DATA_P4K_B` name them; the tests over two Archives skip without the second.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{
    ChangeColumn, ComparisonLevels, ComparisonSide, ComparisonStatus, ComparisonTab, Language,
    Model, ReportStatus,
};
use p4kx_testkit::{real_archive_pair, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const RUN_BUDGET: Duration = Duration::from_secs(60);
const VIEW_BUDGET: Duration = Duration::from_millis(500);

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
    let Some((old, new)) = real_archive_pair()? else {
        return Ok(());
    };
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old);
    tab.set_archive(ComparisonSide::New, new);

    assert!(tab.start(|| {}), "started");
    wait(tab, |tab| !tab.is_running())?;
    let view = tab.view_mut().ok_or("no result")?;
    let mut sortings = Vec::new();
    for column in [
        ChangeColumn::NewSize,
        ChangeColumn::NewSize,
        ChangeColumn::OldSize,
        ChangeColumn::OldSize,
    ] {
        let sorted_at = Instant::now();
        view.sort_by(column);
        sortings.push(sorted_at.elapsed());
    }
    let sorting = sortings.iter().copied().min().unwrap_or(Duration::MAX);
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
    assert!(
        sorting < VIEW_BUDGET,
        "the fastest of {sortings:?} was {sorting:?}"
    );
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
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
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

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn two_real_archives_compare_on_both_levels_and_save_the_report() -> TestResult {
    let Some((old, new)) = real_archive_pair()? else {
        return Ok(());
    };
    let out = tempfile::tempdir()?;
    let report = out.path().join("report.json");
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old);
    tab.set_archive(ComparisonSide::New, new);
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
