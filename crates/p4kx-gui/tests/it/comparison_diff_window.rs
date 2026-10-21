//! The line diff of the Comparison tab drawn and driven headless over a pair of synthetic
//! Archives.

use crate::diffing;
use crate::long_diff;

use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use diffing::{ADDED, BROKEN, EQUAL, Pair, TOO_LARGE, TestResult, pair};
use egui::Key;
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
        .with_size([1280.0, 1000.0])
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

/// Selects the change at `path` the way a click on its row does and waits for its line diff.
fn select(harness: &mut Harness<'_, State>, path: &str) -> TestResult {
    let view = harness
        .state_mut()
        .0
        .comparison_mut()
        .view_mut()
        .ok_or("no result")?;
    let id = view
        .shown()
        .iter()
        .copied()
        .find(|&id| view.change(id).is_some_and(|change| change.path() == path))
        .ok_or("no such change")?;
    view.select(id);
    settle(harness, diff_loaded)
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
        "Difference 1 of 1",
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
            .get_by_label("Next difference")
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

    let first = harness.query_by_label("Difference 1 of 2").is_some();
    harness.get_by_label("Next difference").click();
    harness.run();
    let second = harness.query_by_label("Difference 2 of 2").is_some();
    let shown = harness
        .query_by_label("Lines 32–38 | Lines 32–38")
        .is_some();
    harness.get_by_label("Previous difference").click();
    harness.run();

    assert!(first, "starts at the first hunk");
    assert!(second, "moved to the second hunk");
    assert!(shown, "the second hunk scrolled into view");
    assert!(
        harness.query_by_label("Difference 1 of 2").is_some(),
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
        "Unterschied 1 von 1",
        "Nächster Unterschied",
        "Vorheriger Unterschied",
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    Ok(())
}

#[test]
fn messages_replace_the_diff_where_there_is_none() -> TestResult {
    let (dir, pair) = pair()?;
    let mut harness = compared((dir.path(), &pair), Language::BritishEnglish)?;

    select(&mut harness, TOO_LARGE)?;
    let too_large = harness
        .query_by_label_contains("is too large for a line diff")
        .is_some();
    select(&mut harness, BROKEN)?;
    let failed = harness
        .query_by_label_contains(&format!("Cannot diff {BROKEN}: "))
        .is_some();
    select(&mut harness, EQUAL)?;
    let equal = harness
        .query_by_label("The lines of both sides are equal; only the stored bytes differ.")
        .is_some();
    select(&mut harness, ADDED)?;
    let added = harness.query_by_label("Added line 1: x").is_some();

    assert!(too_large, "too large");
    assert!(failed, "failed with the reason");
    assert!(equal, "equal lines");
    assert!(added, "an added Entry beside an empty old side");
    Ok(())
}

#[test]
fn next_onto_the_last_hunk_hands_the_focus_to_previous() -> TestResult {
    let (dir, pair) = pair()?;
    let mut harness = compared((dir.path(), &pair), Language::BritishEnglish)?;
    open(&mut harness, "Changed: Data\\far.txt")?;

    harness.get_by_label("Next difference").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();

    assert!(
        harness.query_by_label("Difference 2 of 2").is_some(),
        "at the last hunk"
    );
    assert!(
        harness
            .get_by_label("Previous difference")
            .accesskit_node()
            .is_focused(),
        "the focus moved to Previous"
    );
    Ok(())
}

#[test]
fn the_focused_line_diff_scrolls_with_the_keyboard() -> TestResult {
    let (dir, pair) = long_diff::pair()?;
    let mut harness = compared((dir.path(), &pair), Language::BritishEnglish)?;
    open(&mut harness, "Changed: Data\\long.txt")?;
    let added = |line: u32| format!("Added line {line}: changed line {line}");
    let (first, last) = (added(1), added(long_diff::LINES));
    let head = "Lines 1–300 | Lines 1–300";
    assert!(harness.query_by_label(&first).is_some(), "the first line");
    assert!(harness.query_by_label(&last).is_none(), "the last is below");

    harness.get_by_label("Line differences").focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    assert!(
        harness.query_by_label(&last).is_some(),
        "End shows the last"
    );
    assert!(
        harness.query_by_label(&first).is_none(),
        "the first is gone"
    );

    harness.key_press(Key::Home);
    harness.run();
    assert!(
        harness.query_by_label(&first).is_some(),
        "Home shows the first"
    );

    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.query_by_label(head).is_none(), "one row down");
    assert!(harness.query_by_label(&first).is_some(), "only one row");

    harness.key_press(Key::PageDown);
    harness.run();
    assert!(harness.query_by_label(&first).is_none(), "a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!(harness.query_by_label(head).is_some(), "back at the top");
    Ok(())
}
