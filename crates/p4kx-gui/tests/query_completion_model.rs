//! The completion schema of the Query editor lives in the one DataForge cache of the Archive: it is
//! built from the DataForge already read, once, and dropped with the cache.

#[path = "previewing_records/model.rs"]
#[expect(
    dead_code,
    reason = "each test file uses a part of the shared Record helpers"
)]
mod model;
#[expect(
    dead_code,
    reason = "each test file uses a part of the shared Record fixtures"
)]
mod previewing_records;

use std::time::Duration;

use model::{opened, previewed, select, wait_until};
use p4kx_gui::{Browser, Model};
use previewing_records::{ITEM_PATH, MAKER_PATH, TestResult, archive, database};

fn reads(model: &Model) -> Option<usize> {
    model.browser().map(Browser::dataforge_reads)
}

fn builds(model: &Model) -> Option<usize> {
    model.browser().map(Browser::query_schema_builds)
}

fn complete(model: &mut Model) -> TestResult {
    model.prepare_query_completion(|| {});
    wait_until(model, |model| builds(model) == Some(1))
}

#[test]
fn completion_after_the_tree_a_preview_and_a_query_does_not_read_dataforge_again() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, ITEM_PATH)?;
    previewed(&mut model)?;
    model.query_mut().set_text("from Item select speed");
    model.run_query(|| {});
    wait_until(&mut model, |model| !model.query().is_running())?;
    let before = reads(&model);

    complete(&mut model)?;

    assert_eq!(
        before,
        Some(1),
        "the tree, the preview and the Query share one read"
    );
    assert_eq!(reads(&model), Some(1), "completion reads nothing more");
    assert_eq!(builds(&model), Some(1), "the schema is built");
    Ok(())
}

#[test]
fn completion_before_anything_else_reads_dataforge_once_for_all() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    model.query_mut().set_text("from Item select speed");

    complete(&mut model)?;
    model.run_query(|| {});
    wait_until(&mut model, |model| !model.query().is_running())?;
    select(&mut model, MAKER_PATH)?;
    previewed(&mut model)?;

    assert_eq!(
        reads(&model),
        Some(1),
        "one read for completion, Query and preview"
    );
    assert_eq!(builds(&model), Some(1), "still one schema");
    Ok(())
}

#[test]
fn the_schema_is_built_once_however_often_completion_is_prepared() -> TestResult {
    let (_dir, _, mut model) = opened()?;

    for _ in 0..8 {
        model.prepare_query_completion(|| {});
    }
    wait_until(&mut model, |model| builds(model) == Some(1))?;
    for _ in 0..8 {
        model.prepare_query_completion(|| {});
        std::thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(builds(&model), Some(1), "built once");
    assert_eq!(reads(&model), Some(1), "read once");
    Ok(())
}

#[test]
fn a_change_on_disk_drops_the_schema_with_the_cache() -> TestResult {
    let (_dir, path, mut model) = opened()?;
    complete(&mut model)?;

    let (_other, rewritten) = archive(&database().record(p4kx_testkit::TestRecord::new(
        0x9,
        "Maker",
        "Maker.New",
        "libs/makers/new.xml",
    )))?;
    std::fs::copy(rewritten, &path)?;
    let noticed = model.check_archive();
    let dropped = builds(&model);
    model.prepare_query_completion(|| {});
    std::thread::sleep(Duration::from_millis(300));

    assert!(noticed, "the change is noticed");
    assert_eq!(dropped, Some(0), "dropped with the cache");
    assert_eq!(
        builds(&model),
        Some(0),
        "no schema is built for an Archive that changed on disk"
    );
    Ok(())
}
