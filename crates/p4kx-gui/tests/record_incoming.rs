//! The "Referenced by" list of a Record over a synthetic DataForge, driven without drawing: the
//! referring Records with their fields, a Record nobody refers to, following a link, and the one
//! index of References per Archive.

#[path = "previewing_records/model.rs"]
mod model;
mod previewing_records;

use std::error::Error;

use model::{opened, previewed, select, selected_path, wait_until};
use p4kx_gui::{Browser, IncomingStatus, IncomingView, LineValue, Model};
use previewing_records::{ITEM_PATH, MAKER_PATH, SHIP_PATH, TestResult};

fn listed(model: &mut Model) -> Result<&IncomingView, Box<dyn Error>> {
    model
        .browser_mut()
        .ok_or("no Archive")?
        .load_incoming(|| {});
    wait_until(model, |model| {
        model
            .browser()
            .and_then(Browser::incoming_preview)
            .is_some_and(|list| !matches!(list.status(), IncomingStatus::Loading))
    })?;
    match model
        .browser()
        .and_then(Browser::incoming_preview)
        .ok_or("no list")?
        .status()
    {
        IncomingStatus::Ready(view) => Ok(view),
        other => Err(format!("not found: {other:?}").into()),
    }
}

fn rows(view: &IncomingView) -> Vec<(String, String)> {
    view.lines()
        .iter()
        .filter_map(|line| match line.value() {
            LineValue::Reference(link) => {
                Some((line.name().to_owned(), link.record_path().to_owned()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_list_names_each_referring_record_and_the_field_that_holds_the_reference() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, MAKER_PATH)?;

    let view = listed(&mut model)?;

    assert_eq!(
        rows(view),
        [("maker".to_owned(), ITEM_PATH.to_owned())],
        "the Item refers to the Maker through its field maker"
    );
    assert_eq!((view.references(), view.records()), (1, 1), "the counts");
    assert!(
        !view.is_truncated() && !view.is_incomplete(),
        "nothing is left out"
    );
    Ok(())
}

#[test]
fn a_record_nobody_refers_to_has_an_empty_list() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, SHIP_PATH)?;

    let view = listed(&mut model)?;

    assert!(view.lines().is_empty(), "no line");
    assert_eq!((view.references(), view.records()), (0, 0), "the counts");
    Ok(())
}

#[test]
fn following_a_line_selects_the_referring_record() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, MAKER_PATH)?;
    let link = match listed(&mut model)?
        .lines()
        .first()
        .map(p4kx_gui::FieldLine::value)
    {
        Some(LineValue::Reference(link)) => link.clone(),
        other => return Err(format!("no link: {other:?}").into()),
    };

    model.browser_mut().ok_or("no Archive")?.follow(&link);

    assert_eq!(
        selected_path(&model).as_deref(),
        Some(ITEM_PATH),
        "the Item"
    );
    let name = previewed(&mut model)?
        .view()
        .map(p4kx_gui::RecordView::name);
    assert_eq!(name, Some("Item.Sample"), "the Record preview follows");
    let view = listed(&mut model)?;
    assert_eq!(
        rows(view),
        [("cargo".to_owned(), SHIP_PATH.to_owned())],
        "the Ship refers to the Item, found for the new Record"
    );
    Ok(())
}

#[test]
fn the_index_of_references_is_built_once_per_archive() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    for path in [MAKER_PATH, ITEM_PATH, SHIP_PATH, MAKER_PATH] {
        select(&mut model, path)?;
        listed(&mut model)?;
    }

    let browser = model.browser().ok_or("no Archive")?;

    assert_eq!(browser.reference_builds(), 1, "one build for four lists");
    assert_eq!(browser.dataforge_reads(), 1, "the DataForge is read once");
    Ok(())
}
