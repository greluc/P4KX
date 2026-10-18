//! A change of the browsed Archive on disk next to an Extraction of a Comparison side, without
//! drawing.

use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use p4kx_gui::{
    ComparisonSide, ComparisonStatus, Extraction, ExtractionScope, ExtractionStatus, Language,
    Model, TargetKind,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const LARGE_LEN: usize = 4 << 20;
const LARGE_COUNT: usize = 32;

fn poll_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done(model) {
        if Instant::now() > deadline {
            return Err(format!("did not settle: {model:?}").into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        model.comparison_mut().poll();
        model.poll_extraction();
    }
    Ok(())
}

fn write(path: &Path, entries: impl IntoIterator<Item = TestEntry>) -> TestResult {
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(path)?;
    Ok(())
}

fn rewrite(path: &Path, version: usize) -> TestResult {
    write(
        path,
        (0..=version).map(|index| TestEntry::stored(format!("Data\\patch{index}.txt"), "patch")),
    )
}

#[test]
fn a_change_of_the_browsed_archive_closes_only_its_own_setup_and_extraction() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (old, new, browsed) = (
        dir.path().join("old.p4k"),
        dir.path().join("new.p4k"),
        dir.path().join("browsed.p4k"),
    );
    write(&old, [TestEntry::stored("Data\\same.txt", "same")])?;
    write(
        &new,
        (0..LARGE_COUNT).map(|index| {
            TestEntry::stored(format!("Data\\large{index:02}.bin"), vec![7; LARGE_LEN])
        }),
    )?;
    rewrite(&browsed, 0)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(browsed.clone(), || {});
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old);
    tab.set_archive(ComparisonSide::New, new);
    tab.start(|| {});
    poll_until(&mut model, |model| {
        model.browser().is_some()
            && matches!(model.comparison().status(), ComparisonStatus::Finished(_))
    })?;

    model.begin_extraction(ExtractionScope::All);
    rewrite(&browsed, 1)?;
    let browser_setup_changed = model.check_archive();
    let browser_setup_closed = model.extraction_setup().is_none();

    let scope = ExtractionScope::Changes(ComparisonSide::New);
    model.begin_extraction(scope);
    rewrite(&browsed, 2)?;
    let changes_setup_kept = model.check_archive()
        && model
            .extraction_setup()
            .is_some_and(|setup| setup.scope() == scope);
    let setup = model.extraction_setup_mut().ok_or("no setup")?;
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(dir.path().join("out"));
    let started = model.submit_extraction(|| {});
    let running = model.extraction().is_some_and(Extraction::is_running);
    rewrite(&browsed, 3)?;
    let changed_again = model.check_archive();
    let cancelled = model.extraction().is_some_and(Extraction::is_cancelled);
    let stopped = model.change_stopped_extraction();
    poll_until(&mut model, |model| {
        !model.extraction().is_some_and(Extraction::is_running)
    })?;

    assert!(browser_setup_changed, "the browsed Archive changed");
    assert!(
        browser_setup_closed,
        "a setup of the browsed Archive closes"
    );
    assert!(
        changes_setup_kept,
        "the setup of a Comparison side stays open"
    );
    assert!(started && running, "the Extraction runs during the change");
    assert!(changed_again, "the browsed Archive changed again");
    assert!(
        !cancelled,
        "the Extraction of the other Archive is not cancelled"
    );
    assert!(!stopped, "the notice does not claim a stopped Extraction");
    let Some(ExtractionStatus::Finished(summary)) = model.extraction().map(Extraction::status)
    else {
        return Err("no summary".into());
    };
    assert_eq!(summary.extracted(), LARGE_COUNT, "every Entry written");
    Ok(())
}
