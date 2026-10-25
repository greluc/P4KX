// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record list of the Comparison tab drawn and driven headless over a pair of synthetic
//! DataForges, in both languages.

use crate::record_pair;
use crate::waiting::{RunQuiet, wait_for_model};

use std::error::Error;
use std::path::Path;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{
    ComparisonList, ComparisonSide, ComparisonStatus, ComparisonView, Language, Model,
    RecordChangeView, RecordDetailStatus, WorkTab, ui,
};
use record_pair::{BOTH, TestResult, pair};

fn harness((old, new): (&Path, &Path), language: Language) -> Harness<'static, Model> {
    let mut model = Model::new(language);
    model.set_work_tab(WorkTab::Comparison);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old.to_path_buf());
    tab.set_archive(ComparisonSide::New, new.to_path_buf());
    tab.set_levels(BOTH);
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1280.0, 1400.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

fn settle(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    wait_for_model(harness, "the window to settle", done)?;
    harness.run_quiet();
    Ok(())
}

fn detail_loaded(model: &Model) -> bool {
    model
        .comparison()
        .view()
        .and_then(ComparisonView::records)
        .and_then(RecordChangeView::detail)
        .is_some_and(|detail| !matches!(detail.status(), RecordDetailStatus::Loading))
}

fn records_shown(
    pair: (&Path, &Path),
    language: Language,
    (compare, records): (&str, &str),
) -> Result<Harness<'static, Model>, Box<dyn Error>> {
    let mut harness = harness(pair, language);
    harness.run_quiet();
    harness.get_by_label(compare).click();
    settle(&mut harness, |model| {
        matches!(model.comparison().status(), ComparisonStatus::Finished(_))
    })?;
    harness.get_by_role_and_label(Role::Tab, records).click();
    harness.run_quiet();
    Ok(harness)
}

fn open_row(harness: &mut Harness<'_, Model>, label: &str) -> TestResult {
    harness.get_by_label(label).click();
    settle(harness, detail_loaded)
}

#[test]
fn the_records_tab_lists_every_change_with_its_chip_and_record_path() -> TestResult {
    let (_dir, old, new) = pair()?;

    let harness = records_shown(
        (&old, &new),
        Language::BritishEnglish,
        ("Compare", "Records"),
    )?;

    assert_eq!(
        harness
            .state()
            .comparison()
            .view()
            .map(ComparisonView::list),
        Some(ComparisonList::Records),
        "the tab switched the list"
    );
    for row in [
        "Added: libs/ships/fresh",
        "Removed: libs/makers/gone",
        "Moved: libs/ships/wanderer to libs/ships/moved/wanderer",
        "Changed: libs/items/sample",
        "Changed: libs/ships/tuned",
        "Unreadable: libs/broken/one",
    ] {
        assert!(harness.query_by_label(row).is_some(), "the row {row}");
    }
    assert!(
        harness.query_by_label("8 shown").is_some(),
        "the Records are counted"
    );
    for head in ["Change", "Record path", "Type"] {
        assert!(harness.query_by_label(head).is_some(), "the head {head}");
    }
    Ok(())
}

#[test]
fn extracting_the_changes_of_a_side_belongs_to_the_entries_tab() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut harness = records_shown(
        (&old, &new),
        Language::BritishEnglish,
        ("Compare", "Records"),
    )?;

    let on_records = harness.query_by_label_contains("Extract new").is_some();
    harness.get_by_role_and_label(Role::Tab, "Entries").click();
    harness.run_quiet();
    let on_entries = harness.query_by_label_contains("Extract new").is_some();

    assert!(!on_records, "no Extraction of Entries beside the Records");
    assert!(on_entries, "the Entries keep their Extraction");
    Ok(())
}

#[test]
fn a_changed_record_opens_its_field_changes_below_the_list() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut harness = records_shown(
        (&old, &new),
        Language::BritishEnglish,
        ("Compare", "Records"),
    )?;

    open_row(&mut harness, "Changed: libs/items/sample")?;

    for row in [
        "maker: libs/makers/one to libs/makers/two",
        "hull.armor: 1 to 2",
        "focus: libs/items/sample · nodes[0] to libs/items/sample · nodes[1]",
    ] {
        assert!(harness.query_by_label(row).is_some(), "the field row {row}");
    }
    let label = harness
        .query_by_label_contains("hull.label: Hull to Error: ")
        .map(|node| node.accesskit_node().label().unwrap_or_default());
    assert!(
        label
            .as_deref()
            .is_some_and(|text| text.len() > "hull.label: Hull to Error: ".len()),
        "the error value with its reason: {label:?}"
    );
    assert!(
        harness.query_by_label("Field Old New").is_some(),
        "the head"
    );
    assert!(
        harness.query_by_label("libs/items/sample").is_some(),
        "the path line"
    );
    Ok(())
}

#[test]
fn an_unreadable_moved_or_added_record_says_why_it_has_no_field_rows() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut harness = records_shown(
        (&old, &new),
        Language::BritishEnglish,
        ("Compare", "Records"),
    )?;

    open_row(&mut harness, "Unreadable: libs/broken/one")?;
    let unreadable = harness
        .query_by_label_contains("The new Record cannot be read: ")
        .is_some();
    open_row(
        &mut harness,
        "Moved: libs/ships/wanderer to libs/ships/moved/wanderer",
    )?;
    let moved = harness
        .query_by_label("Only the record path changed; the fields are equal.")
        .is_some();
    open_row(&mut harness, "Added: libs/ships/fresh")?;
    let added = harness
        .query_by_label("The Record exists only in the new Archive.")
        .is_some();

    assert!(unreadable, "the reason of the unreadable side");
    assert!(moved, "the moved Record");
    assert!(added, "the added Record");
    Ok(())
}

#[test]
fn the_record_list_and_its_fields_speak_german() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut harness = records_shown((&old, &new), Language::German, ("Vergleichen", "Records"))?;

    let moved = harness
        .query_by_label("Verschoben: libs/ships/wanderer zu libs/ships/moved/wanderer")
        .is_some();
    let unreadable = harness
        .query_by_label("Unlesbar: libs/broken/one")
        .is_some();
    open_row(&mut harness, "Geändert: libs/ships/tuned")?;

    assert!(moved, "moved in German");
    assert!(unreadable, "unreadable in German");
    assert!(
        harness.query_by_label("speed: 4 zu 40").is_some(),
        "the field row"
    );
    assert!(
        harness.query_by_label("Feld Alt Neu").is_some(),
        "the German head"
    );
    for head in ["Änderung", "Record-Pfad", "Typ"] {
        assert!(harness.query_by_label(head).is_some(), "the head {head}");
    }
    Ok(())
}

#[test]
fn the_filter_narrows_the_record_list() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut harness = records_shown(
        (&old, &new),
        Language::BritishEnglish,
        ("Compare", "Records"),
    )?;

    harness.get_by_role(Role::TextInput).click();
    harness.run_quiet();
    harness
        .get_by_role(Role::TextInput)
        .type_text("makers/gone");
    settle(&mut harness, |model| {
        model
            .comparison()
            .view()
            .and_then(ComparisonView::records)
            .is_some_and(|records| !records.is_filtering() && records.shown().len() == 1)
    })?;

    assert!(
        harness
            .query_by_label("Removed: libs/makers/gone")
            .is_some(),
        "the matching Record"
    );
    assert!(
        harness
            .query_by_label("Changed: libs/ships/tuned")
            .is_none(),
        "the others are hidden"
    );
    assert!(
        harness.query_by_label("1 shown").is_some(),
        "the count follows"
    );
    Ok(())
}

#[test]
fn the_records_tab_is_missing_without_the_record_level() -> TestResult {
    let (_dir, old, new) = pair()?;
    let mut harness = harness((&old, &new), Language::BritishEnglish);
    harness
        .state_mut()
        .comparison_mut()
        .set_levels(p4kx_gui::ComparisonLevels::default());
    harness.run_quiet();

    harness.get_by_label("Compare").click();
    settle(&mut harness, |model| {
        matches!(model.comparison().status(), ComparisonStatus::Finished(_))
    })?;

    assert!(
        harness
            .query_by_role_and_label(Role::Tab, "Records")
            .is_none(),
        "no Records tab"
    );
    Ok(())
}
