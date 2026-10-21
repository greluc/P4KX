//! The Record preview over a synthetic DataForge, driven without drawing: fields, error values,
//! following References, back and forward, and the one DataForge cache of the Archive.

use crate::previewing_records;
use crate::previewing_records::model;

use std::error::Error;
use std::time::{Duration, Instant};

use model::{opened, previewed, select, selected_path, wait_until};
use p4kx_gui::{
    Browser, FieldLine, LineValue, Model, RecordExport, RecordExportStatus, RecordForm, RecordLink,
    RecordPreviewStatus, RecordView, TargetKind,
};
use previewing_records::{ITEM_PATH, MAKER_PATH, SHIP_PATH, TestResult, archive, database};

fn view(model: &mut Model) -> Result<&RecordView, Box<dyn Error>> {
    let preview = previewed(model)?;
    match preview.status() {
        RecordPreviewStatus::Ready(view) => Ok(view),
        other => Err(format!("not read: {other:?}").into()),
    }
}

fn shown(field: &FieldLine) -> String {
    let value = match field.value() {
        LineValue::Scalar(text) => text.to_string(),
        LineValue::Key { key, text } => format!("key {key} {text:?}"),
        LineValue::Struct(type_name) => format!("struct {type_name}"),
        LineValue::Array(len) => format!("array {len}"),
        LineValue::Null => "null".to_owned(),
        LineValue::Reference(link) => format!("-> {} ({})", link.record_path(), link.type_name()),
        LineValue::Dangling { guid, .. } => format!("dangling {guid}"),
        LineValue::Error(_) => "error".to_owned(),
    };
    format!(
        "{}{} = {value}",
        "  ".repeat(usize::from(field.depth())),
        field.name()
    )
}

fn link(model: &mut Model, field: &str) -> Result<RecordLink, Box<dyn Error>> {
    let view = view(model)?;
    view.lines()
        .iter()
        .find_map(|line| match line.value() {
            LineValue::Reference(link) if line.name() == field => Some(link.clone()),
            _ => None,
        })
        .ok_or_else(|| format!("no link {field}").into())
}

fn follow(model: &mut Model, field: &str) -> TestResult {
    let link = link(model, field)?;
    model.browser_mut().ok_or("no Archive")?.follow(&link);
    Ok(())
}

fn browser(model: &mut Model) -> Result<&mut Browser, Box<dyn Error>> {
    model.browser_mut().ok_or_else(|| "no Archive".into())
}

#[test]
fn a_selected_record_shows_its_type_guid_path_and_every_field_with_owned_children_nested()
-> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, ITEM_PATH)?;

    let view = view(&mut model)?;

    assert_eq!(view.type_name(), "Item", "type");
    assert_eq!(
        view.guid().to_string(),
        "70000000-0000-4000-8000-000000000002",
        "GUID"
    );
    assert_eq!(view.record_path(), "libs/items/sample", "record path");
    assert_eq!(view.name(), "Item.Sample", "name");
    let lines: Vec<String> = view.lines().iter().map(shown).collect();
    assert_eq!(
        lines,
        [
            "maker = -> libs/makers/one (Maker)",
            "spare = dangling 70000000-0000-4000-8000-0000000000ff",
            "nodes = array 2",
            "  [0] = struct Node",
            "    weight = 1",
            "  [1] = struct Node",
            "    weight = 2",
            "hull = struct Hull",
            "  label = error",
            "  armor = 3",
            "speed = 5",
        ],
        "every field depth first"
    );
    let xml = String::from_utf8_lossy(view.xml().bytes()).into_owned();
    assert!(xml.contains("<Item"), "Record XML: {xml}");
    Ok(())
}

#[test]
fn an_error_value_carries_its_reason_and_the_rest_of_the_record_is_shown() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, ITEM_PATH)?;

    let view = view(&mut model)?;

    let label = view
        .lines()
        .iter()
        .find(|line| line.name() == "label")
        .ok_or("no label")?;
    let LineValue::Error(reason) = label.value() else {
        return Err(format!("not an error: {label:?}").into());
    };
    assert!(
        reason.contains("not a valid string"),
        "the reason: {reason}"
    );
    assert_eq!(
        view.error_count(),
        2,
        "the label and the dangling Reference"
    );
    assert!(
        view.lines().iter().any(|line| line.name() == "speed"),
        "the fields after it"
    );
    Ok(())
}

#[test]
fn following_a_reference_selects_the_target_in_the_tree_and_the_preview() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, ITEM_PATH)?;

    follow(&mut model, "maker")?;
    let view = view(&mut model)?;

    assert_eq!(view.record_path(), MAKER_PATH, "the Maker is previewed");
    assert_eq!(view.lines().len(), 1, "its rank");
    assert_eq!(
        selected_path(&model).as_deref(),
        Some(MAKER_PATH),
        "and selected in the tree"
    );
    Ok(())
}

#[test]
fn back_and_forward_return_along_several_followed_references() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, SHIP_PATH)?;
    follow(&mut model, "cargo")?;
    follow(&mut model, "maker")?;

    let at_end = (
        browser(&mut model)?.can_go_back(),
        browser(&mut model)?.can_go_forward(),
    );
    browser(&mut model)?.go_back();
    let once = selected_path(&model);
    browser(&mut model)?.go_back();
    let twice = selected_path(&model);
    let at_start = browser(&mut model)?.can_go_back();
    browser(&mut model)?.go_forward();
    let forward = selected_path(&model);
    let forward_view = view(&mut model)?.record_path().to_owned();

    assert_eq!(at_end, (true, false), "back only, at the end of the way");
    assert_eq!(once.as_deref(), Some(ITEM_PATH), "one step back");
    assert_eq!(twice.as_deref(), Some(SHIP_PATH), "two steps back");
    assert!(!at_start, "nothing before the first Record");
    assert_eq!(forward.as_deref(), Some(ITEM_PATH), "one step forward");
    assert_eq!(forward_view, ITEM_PATH, "previewed after the step");
    Ok(())
}

#[test]
fn a_new_selection_after_going_back_forgets_the_way_forward() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, SHIP_PATH)?;
    follow(&mut model, "cargo")?;
    browser(&mut model)?.go_back();

    select(&mut model, MAKER_PATH)?;

    assert!(!browser(&mut model)?.can_go_forward(), "no way forward");
    browser(&mut model)?.go_back();
    assert_eq!(selected_path(&model).as_deref(), Some(SHIP_PATH), "back");
    Ok(())
}

#[test]
fn following_a_dangling_reference_notes_it_instead_of_navigating() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, ITEM_PATH)?;
    let view = view(&mut model)?;
    let guid = view
        .lines()
        .iter()
        .find_map(|line| match line.value() {
            LineValue::Dangling { guid, .. } => Some(*guid),
            _ => None,
        })
        .ok_or("no dangling Reference")?;

    browser(&mut model)?.follow_dangling(guid);

    let noted = browser(&mut model)?
        .dangling_reference()
        .map(|guid| guid.to_string());
    assert_eq!(
        noted.as_deref(),
        Some("70000000-0000-4000-8000-0000000000ff"),
        "the missing GUID"
    );
    assert_eq!(
        selected_path(&model).as_deref(),
        Some(ITEM_PATH),
        "no navigation"
    );
    assert!(!browser(&mut model)?.can_go_back(), "no step taken");
    select(&mut model, MAKER_PATH)?;
    assert!(
        browser(&mut model)?.dangling_reference().is_none(),
        "gone with the next Record"
    );
    Ok(())
}

#[test]
fn record_tree_query_and_record_export_share_one_read_of_dataforge() -> TestResult {
    let (_dir, _, mut model) = opened()?;
    select(&mut model, ITEM_PATH)?;
    view(&mut model)?;

    let kept = model.record_export_keeps_dataforge();
    model.query_mut().set_text("from Item select speed");
    model.run_query(|| {});
    wait_until(&mut model, |model| !model.query().is_running())?;
    let out = tempfile::tempdir()?;
    model.begin_record_export();
    let setup = model.record_export_setup_mut().ok_or("no setup")?;
    setup.set_form(RecordForm::Json);
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out.path().to_path_buf());
    setup.set_filter("*items*");
    let started = model.submit_record_export(|| {});
    let deadline = Instant::now() + Duration::from_secs(30);
    while model.record_export().is_none_or(RecordExport::is_running) {
        if Instant::now() > deadline {
            return Err("the export did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll_record_export();
    }
    let exported = match model.record_export().map(RecordExport::status) {
        Some(RecordExportStatus::Finished(summary)) => summary.written(),
        other => return Err(format!("the export did not finish: {other:?}").into()),
    };
    let reads = model.browser().map(Browser::dataforge_reads);

    assert!(kept, "the Record tree read it for everyone");
    assert!(model.query().result().is_some(), "the Query ran");
    assert!(started, "the export started");
    assert_eq!(exported, 1, "the export wrote the Item");
    assert_eq!(
        reads,
        Some(1),
        "one read for the tree, the preview, the Query and the export"
    );
    Ok(())
}

#[test]
fn a_change_on_disk_drops_the_preview_the_way_taken_and_the_shared_dataforge() -> TestResult {
    let (_dir, path, mut model) = opened()?;
    select(&mut model, SHIP_PATH)?;
    follow(&mut model, "cargo")?;
    view(&mut model)?;

    let (_other, rewritten) = archive(&database().record(p4kx_testkit::TestRecord::new(
        0x9,
        "Maker",
        "Maker.New",
        "libs/makers/new.xml",
    )))?;
    std::fs::copy(rewritten, &path)?;
    let noticed = model.check_archive();
    wait_until(&mut model, |_| true)?;

    let browser = model.browser().ok_or("no Archive")?;
    assert!(noticed, "noticed");
    assert!(!browser.can_go_back(), "the way taken is forgotten");
    assert!(browser.record_preview().is_none(), "no Record is selected");
    assert!(
        !model.record_export_keeps_dataforge(),
        "DataForge is dropped"
    );
    Ok(())
}
