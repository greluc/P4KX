//! The Record list of the Comparison tab over a pair of synthetic DataForges: change kinds,
//! record paths, types, sorting, the Filter and the field changes of one Record.

mod record_pair;

use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use p4kx_comparison::{RecordChangeKind, RecordSide};
use p4kx_gui::{
    ComparisonLevels, ComparisonList, ComparisonSide, ComparisonStatus, ComparisonTab,
    ComparisonView, FieldName, FieldRow, FieldValue, Language, Model, RecordChangeColumn,
    RecordChangeView, RecordDetailStatus, RecordFields, SortDirection,
};
use record_pair::{BOTH, TestResult, pair};

/// Polls `tab` until `done` holds.
fn wait(tab: &mut ComparisonTab, done: impl Fn(&ComparisonTab) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(tab) {
        if Instant::now() > deadline {
            return Err(format!("the Comparison did not settle: {:?}", tab.status()).into());
        }
        std::thread::sleep(Duration::from_millis(5));
        tab.poll();
    }
    Ok(())
}

/// Compares `old` with `new` on `levels` and waits until it finished.
fn compared((old, new): (&Path, &Path), levels: ComparisonLevels) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old.to_path_buf());
    tab.set_archive(ComparisonSide::New, new.to_path_buf());
    tab.set_levels(levels);
    if !tab.start(|| {}) {
        return Err("the Comparison did not start".into());
    }
    wait(tab, |tab| !tab.is_running())?;
    match tab.status() {
        ComparisonStatus::Finished(_) => Ok(model),
        other => Err(format!("not finished: {other:?}").into()),
    }
}

type Row = (RecordChangeKind, Option<String>, Option<String>, String);

fn rows(records: &RecordChangeView) -> Vec<Row> {
    records
        .shown()
        .iter()
        .filter_map(|&id| records.row(id))
        .map(|row| {
            (
                row.kind(),
                row.old_path().map(str::to_owned),
                row.new_path().map(str::to_owned),
                row.type_name().to_owned(),
            )
        })
        .collect()
}

fn paths(records: &RecordChangeView) -> Vec<String> {
    records
        .shown()
        .iter()
        .filter_map(|&id| records.row(id))
        .map(|row| row.path().to_owned())
        .collect()
}

fn records(model: &Model) -> Result<&RecordChangeView, Box<dyn Error>> {
    model
        .comparison()
        .view()
        .and_then(ComparisonView::records)
        .ok_or_else(|| "no Record list".into())
}

fn records_mut(model: &mut Model) -> Result<&mut RecordChangeView, Box<dyn Error>> {
    model
        .comparison_mut()
        .view_mut()
        .and_then(ComparisonView::records_mut)
        .ok_or_else(|| "no Record list".into())
}

fn fields_of(model: &mut Model, path: &str) -> Result<RecordFields, Box<dyn Error>> {
    let list = records(model)?;
    let id = list
        .shown()
        .iter()
        .copied()
        .find(|&id| list.row(id).is_some_and(|row| row.path() == path))
        .ok_or_else(|| format!("no row {path}"))?;
    records_mut(model)?.select(id);
    let view = model.comparison_mut().view_mut().ok_or("no view")?;
    view.load_record_detail(|| {});
    wait(model.comparison_mut(), |tab| {
        tab.view()
            .and_then(ComparisonView::records)
            .and_then(RecordChangeView::detail)
            .is_some_and(|detail| !matches!(detail.status(), RecordDetailStatus::Loading))
    })?;
    match records(model)?.detail().map(|detail| detail.status()) {
        Some(RecordDetailStatus::Ready(fields)) => Ok(fields.clone()),
        other => Err(format!("no field changes: {other:?}").into()),
    }
}

fn field(name: &str, old: Option<FieldValue>, new: Option<FieldValue>) -> FieldRow {
    FieldRow {
        name: FieldName::Field(name.into()),
        old,
        new,
    }
}

#[test]
fn the_record_list_shows_kind_paths_and_type_in_path_order() -> TestResult {
    let (_dir, old, new) = pair()?;

    let model = compared((&old, &new), BOTH)?;

    let records = records(&model)?;
    assert_eq!(
        rows(records),
        [
            (
                RecordChangeKind::Unreadable,
                Some("libs/broken/one".to_owned()),
                Some("libs/broken/one".to_owned()),
                "Broken".to_owned()
            ),
            (
                RecordChangeKind::Changed,
                Some("libs/items/sample".to_owned()),
                Some("libs/items/sample".to_owned()),
                "Item".to_owned()
            ),
            (
                RecordChangeKind::Removed,
                Some("libs/makers/gone".to_owned()),
                None,
                "Maker".to_owned()
            ),
            (
                RecordChangeKind::Added,
                None,
                Some("libs/ships/fresh".to_owned()),
                "Ship".to_owned()
            ),
            (
                RecordChangeKind::Moved,
                Some("libs/ships/wanderer".to_owned()),
                Some("libs/ships/moved/wanderer".to_owned()),
                "Ship".to_owned()
            ),
            (
                RecordChangeKind::Changed,
                Some("libs/ships/tuned".to_owned()),
                Some("libs/ships/tuned".to_owned()),
                "Ship".to_owned()
            ),
        ],
        "every kind of change, the moved Record with both paths"
    );
    assert_eq!(
        model.comparison().view().map(ComparisonView::list),
        Some(ComparisonList::Entries),
        "the Entries come first while they were compared"
    );
    Ok(())
}

#[test]
fn the_record_list_sorts_by_change_and_by_type() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    records_mut(&mut model)?.sort_by(RecordChangeColumn::Change);
    let by_change = paths(records(&model)?);
    records_mut(&mut model)?.sort_by(RecordChangeColumn::Change);
    let descending = records(&model)?.sort().direction;
    let by_change_reversed = paths(records(&model)?);
    records_mut(&mut model)?.sort_by(RecordChangeColumn::Type);
    let by_type = paths(records(&model)?);

    assert_eq!(
        by_change,
        [
            "libs/ships/fresh",
            "libs/makers/gone",
            "libs/ships/moved/wanderer",
            "libs/items/sample",
            "libs/ships/tuned",
            "libs/broken/one",
        ],
        "added, removed, moved, changed, unreadable; ties in path order"
    );
    assert_eq!(
        descending,
        SortDirection::Descending,
        "a second click reverses"
    );
    assert_eq!(
        by_change_reversed.first().map(String::as_str),
        Some("libs/broken/one"),
        "unreadable first when descending"
    );
    assert_eq!(
        by_type,
        [
            "libs/broken/one",
            "libs/items/sample",
            "libs/makers/gone",
            "libs/ships/fresh",
            "libs/ships/moved/wanderer",
            "libs/ships/tuned",
        ],
        "Broken, Item, Maker, then the Ships in path order"
    );
    Ok(())
}

#[test]
fn the_filter_selects_records_by_either_record_path() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let view = model.comparison_mut().view_mut().ok_or("no view")?;
    view.set_filter("ships/wanderer", || {});
    wait(model.comparison_mut(), |tab| {
        tab.view()
            .and_then(ComparisonView::records)
            .is_some_and(|records| !records.is_filtering())
    })?;
    let by_old_path = paths(records(&model)?);
    let view = model.comparison_mut().view_mut().ok_or("no view")?;
    view.set_filter("", || {});
    let all = records(&model)?.shown().len();

    assert_eq!(
        by_old_path,
        ["libs/ships/moved/wanderer"],
        "the moved Record matches by its old path"
    );
    assert_eq!(all, 6, "no Filter, every Record");
    Ok(())
}

#[test]
fn a_changed_record_shows_its_field_changes_with_reference_paths_and_error_reasons() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let fields = fields_of(&mut model, "libs/items/sample")?;

    let [maker, label, armor] = fields.rows() else {
        return Err(format!("three field changes: {:?}", fields.rows()).into());
    };
    let reference = |path: &str| {
        Some(FieldValue::Reference {
            record_path: path.into(),
            at: None,
        })
    };
    assert_eq!(
        *maker,
        field(
            "maker",
            reference("libs/makers/one"),
            reference("libs/makers/two")
        ),
        "the Reference by both target record paths"
    );
    assert_eq!(
        label.name,
        FieldName::Field("hull.label".into()),
        "nested path"
    );
    assert_eq!(
        label.old,
        Some(FieldValue::Scalar("Hull".into())),
        "old string"
    );
    assert!(
        matches!(&label.new, Some(FieldValue::Error(reason)) if reason.contains("text")),
        "the error value with its reason: {:?}",
        label.new
    );
    assert_eq!(
        *armor,
        field(
            "hull.armor",
            Some(FieldValue::Scalar("1".into())),
            Some(FieldValue::Scalar("2".into()))
        ),
        "armor"
    );
    assert!(fields.errors().is_empty(), "readable");
    Ok(())
}

#[test]
fn a_scalar_change_shows_old_and_new_value() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let fields = fields_of(&mut model, "libs/ships/tuned")?;

    assert_eq!(
        fields.rows(),
        [field(
            "speed",
            Some(FieldValue::Scalar("4".into())),
            Some(FieldValue::Scalar("40".into()))
        )],
        "speed 4 to 40"
    );
    Ok(())
}

#[test]
fn an_unreadable_record_gives_the_side_and_the_reason() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let fields = fields_of(&mut model, "libs/broken/one")?;

    let sides: Vec<_> = fields.errors().iter().map(|error| error.side).collect();
    assert_eq!(sides, [RecordSide::New], "the new side cannot be read");
    assert!(
        fields.errors().iter().all(|error| !error.reason.is_empty()),
        "with a reason"
    );
    assert!(fields.rows().is_empty(), "no field compared");
    Ok(())
}

#[test]
fn a_moved_and_an_added_record_have_no_field_changes() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let moved = fields_of(&mut model, "libs/ships/moved/wanderer")?;
    let added = fields_of(&mut model, "libs/ships/fresh")?;

    assert!(moved.rows().is_empty(), "only the path moved: {moved:?}");
    assert!(added.rows().is_empty(), "nothing to compare: {added:?}");
    Ok(())
}

#[test]
fn the_record_level_alone_shows_the_record_list_first() -> TestResult {
    let (_dir, old, new) = pair()?;
    let records = ComparisonLevels {
        entries: false,
        records: true,
    };

    let model = compared((&old, &new), records)?;

    let view = model.comparison().view().ok_or("no view")?;
    assert_eq!(view.list(), ComparisonList::Records, "the Record list");
    assert_eq!(view.shown_len(), 6, "its rows are counted");
    Ok(())
}

#[test]
fn without_the_record_level_there_is_no_record_list() -> TestResult {
    let (_dir, old, new) = pair()?;

    let mut model = compared((&old, &new), ComparisonLevels::default())?;

    let view = model.comparison_mut().view_mut().ok_or("no view")?;
    view.show_list(ComparisonList::Records);
    assert!(view.records().is_none(), "no Record list");
    assert_eq!(view.list(), ComparisonList::Entries, "the Entries stay");
    Ok(())
}
