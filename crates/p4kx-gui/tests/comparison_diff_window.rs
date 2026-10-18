//! The line diff of the Comparison tab drawn and driven headless over a pair of synthetic
//! Archives.

mod diffing;

use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use diffing::{Pair, TestResult, pair};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_channels::SearchPaths;
use p4kx_gui::ui::{self, Requests};
use p4kx_gui::{
    ChannelList, ComparisonSide, ComparisonStatus, ComparisonView, DiffStatus, Language, Model,
    WorkTab,
};
use p4kx_settings::ManualChannels;

type State = (Model, Requests);

fn harness((library, pair): (&Path, &Pair), language: Language) -> Harness<'static, State> {
    let mut model = Model::new(language);
    model.set_channels(ChannelList::new(
        SearchPaths {
            default_library: Some(library.to_path_buf()),
            ..SearchPaths::default()
        },
        ManualChannels::default(),
    ));
    model.set_work_tab(WorkTab::Comparison);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, pair.old.clone());
    tab.set_archive(ComparisonSide::New, pair.new.clone());
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1280.0, 900.0])
        .build_ui_state(
            |ui, (model, requests): &mut State| {
                let frame = ui::show(ui, model);
                requests.dialog = requests.dialog.or(frame.dialog);
            },
            (model, Requests::default()),
        )
}

fn settle(harness: &mut Harness<'_, State>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    harness.step();
    while !done(&harness.state().0) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}

fn compared(
    pair: (&Path, &Pair),
    language: Language,
) -> Result<Harness<'static, State>, Box<dyn Error>> {
    let mut harness = harness(pair, language);
    harness.run();
    let start = if language == Language::German {
        "Vergleichen"
    } else {
        "Compare"
    };
    harness.get_by_label(start).click();
    settle(&mut harness, |model| {
        matches!(model.comparison().status(), ComparisonStatus::Finished(_))
    })?;
    Ok(harness)
}

fn diff_loaded(model: &Model) -> bool {
    model
        .comparison()
        .view()
        .and_then(ComparisonView::diff)
        .is_some_and(|diff| !matches!(diff.status(), DiffStatus::Loading))
}

/// Clicks the change row named `row` and waits for its line diff.
fn open(harness: &mut Harness<'_, State>, row: &str) -> TestResult {
    harness.get_by_label(row).click();
    settle(harness, diff_loaded)
}

#[test]
fn selecting_a_changed_text_entry_shows_both_sides_next_to_each_other() -> TestResult {
    let (dir, pair) = pair()?;
    let mut harness = compared((dir.path(), &pair), Language::BritishEnglish)?;

    open(&mut harness, "Changed: Data\\a.cfg")?;

    for label in [
        "Old New",
        "Lines 1–4 | Lines 1–5",
        "Removed line 2: b",
        "Added line 2: B",
        "Added line 5: e",
        "Change 1 of 1",
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    assert_eq!(
        harness.query_all_by_label("Line 1: a").count(),
        2,
        "an unchanged line on both sides"
    );
    assert!(
        harness
            .get_by_label("Next change")
            .accesskit_node()
            .is_disabled(),
        "one hunk only"
    );
    Ok(())
}

#[test]
fn a_changed_cryxml_entry_shows_the_converted_xml() -> TestResult {
    let (dir, pair) = pair()?;
    let mut harness = compared((dir.path(), &pair), Language::BritishEnglish)?;

    open(&mut harness, "Changed: Data\\vehicle.xml")?;

    assert!(
        harness
            .query_by_label("Removed line 3:   <Part damageMax=\"2500\"/>")
            .is_some(),
        "old attribute"
    );
    assert!(
        harness
            .query_by_label("Added line 3:   <Part damageMax=\"3000\"/>")
            .is_some(),
        "new attribute"
    );
    Ok(())
}

#[test]
fn a_changed_binary_entry_shows_the_binary_message() -> TestResult {
    let (dir, pair) = pair()?;
    let mut harness = compared((dir.path(), &pair), Language::BritishEnglish)?;

    open(&mut harness, "Changed: Data\\tex.dds")?;

    assert!(
        harness
            .query_by_label("Data\\tex.dds is binary, so there is no line diff.")
            .is_some(),
        "the binary message"
    );
    Ok(())
}

#[test]
fn next_and_previous_jump_between_the_hunks() -> TestResult {
    let (dir, pair) = pair()?;
    let mut harness = compared((dir.path(), &pair), Language::BritishEnglish)?;
    open(&mut harness, "Changed: Data\\far.txt")?;

    let first = harness.query_by_label("Change 1 of 2").is_some();
    harness.get_by_label("Next change").click();
    harness.run();
    let second = harness.query_by_label("Change 2 of 2").is_some();
    let shown = harness
        .query_by_label("Lines 32–38 | Lines 32–38")
        .is_some();
    harness.get_by_label("Previous change").click();
    harness.run();

    assert!(first, "starts at the first hunk");
    assert!(second, "moved to the second hunk");
    assert!(shown, "the second hunk scrolled into view");
    assert!(
        harness.query_by_label("Change 1 of 2").is_some(),
        "back at the first hunk"
    );
    Ok(())
}

#[test]
fn the_german_diff_speaks_german() -> TestResult {
    let (dir, pair) = pair()?;
    let mut harness = compared((dir.path(), &pair), Language::German)?;

    open(&mut harness, "Geändert: Data\\a.cfg")?;

    for label in [
        "Alt Neu",
        "Entfernte Zeile 2: b",
        "Hinzugefügte Zeile 2: B",
        "Änderung 1 von 1",
        "Nächste Änderung",
        "Vorherige Änderung",
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    Ok(())
}
