//! The field changes of one Record of the Comparison's Record list over a pair of synthetic
//! DataForges: values, links, error values, names and types, and a new selection.

mod record_model;
mod record_pair;

use std::error::Error;

use p4kx_comparison::RecordSide;
use p4kx_gui::{
    ComparisonView, FieldName, FieldRow, FieldValue, Model, RecordChangeView, RecordDetail,
    RecordDetailStatus, RecordFields,
};
use record_model::{compared, records, records_mut, wait};
use record_pair::{BOTH, TestResult, pair};

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
fn a_changed_record_shows_its_field_changes_with_reference_paths_and_error_reasons() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let fields = fields_of(&mut model, "libs/items/sample")?;

    let [maker, focus, label, armor] = fields.rows() else {
        return Err(format!("four field changes: {:?}", fields.rows()).into());
    };
    let reference = |path: &str, at: Option<&str>| {
        Some(FieldValue::Reference {
            record_path: path.into(),
            at: at.map(Into::into),
        })
    };
    assert_eq!(
        *maker,
        field(
            "maker",
            reference("libs/makers/one", None),
            reference("libs/makers/two", None)
        ),
        "the Reference by both target record paths"
    );
    assert_eq!(
        *focus,
        field(
            "focus",
            reference("libs/items/sample", Some("nodes[0]")),
            reference("libs/items/sample", Some("nodes[1]"))
        ),
        "the link into the Record by record path and place"
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
fn a_changed_name_and_type_come_first_as_rows_of_their_own() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let renamed = fields_of(&mut model, "libs/ships/renamed")?;
    let retyped = fields_of(&mut model, "libs/ships/retyped")?;

    assert_eq!(
        renamed.rows(),
        [FieldRow {
            name: FieldName::RecordName,
            old: Some(FieldValue::Scalar("Ship.Before".into())),
            new: Some(FieldValue::Scalar("Ship.After".into())),
        }],
        "only the name changed"
    );
    let names: Vec<&FieldName> = retyped.rows().iter().map(|row| &row.name).collect();
    assert_eq!(
        names,
        [
            &FieldName::RecordType,
            &FieldName::Field("rank".into()),
            &FieldName::Field("speed".into()),
        ],
        "the type first, then the fields of both structs"
    );
    assert_eq!(
        retyped.rows().first().map(|row| (&row.old, &row.new)),
        Some((
            &Some(FieldValue::Scalar("Ship".into())),
            &Some(FieldValue::Scalar("Maker".into()))
        )),
        "old and new type"
    );
    Ok(())
}

#[test]
fn selecting_another_record_replaces_the_field_changes() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut model = compared((&old, &new), BOTH)?;

    let first = fields_of(&mut model, "libs/ships/tuned")?;
    let first_generation = records(&model)?
        .detail()
        .map(RecordDetail::generation)
        .ok_or("no detail")?;
    let second = fields_of(&mut model, "libs/ships/renamed")?;
    let detail = records(&model)?.detail().ok_or("no detail")?;
    let selected = records(&model)?
        .selected()
        .and_then(|id| records(&model).ok()?.row(id))
        .map(|row| row.path().to_owned());

    assert_ne!(first, second, "another Record, other changes");
    assert_ne!(detail.generation(), first_generation, "a new request");
    assert_eq!(
        selected.as_deref(),
        Some("libs/ships/renamed"),
        "the detail follows the selection"
    );
    assert!(
        matches!(detail.status(), RecordDetailStatus::Ready(fields) if *fields == second),
        "the newest answer is shown"
    );
    Ok(())
}
