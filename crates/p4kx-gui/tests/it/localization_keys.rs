//! The localisation tab with the keyboard and the clipboard: Ctrl+C, the selection after a search,
//! scrolling the table, and the heads after another Archive was opened.

use crate::localizing;

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use localizing::{TestResult, archive, both, copied, english, opened, type_search, wait_until};
use p4kx_gui::{Language, LocalizationStatus, Model, TextLanguage};
use p4kx_testkit::GlobalIniBuilder;

fn ctrl_c(harness: &mut egui_kittest::Harness<'_, Model>) {
    harness.event(Event::Copy);
    harness.step();
}

#[test]
fn ctrl_c_copies_the_text_of_the_selected_cell_or_its_key() -> TestResult {
    let (_dir, mut harness) = opened(&both(), Language::BritishEnglish)?;
    ctrl_c(&mut harness);
    assert!(copied(&harness).is_empty(), "nothing selected");

    harness.get_by_label("Gewehr").click();
    harness.run_steps(2);
    ctrl_c(&mut harness);
    assert_eq!(copied(&harness), ["Gewehr"], "the text of the cell");

    harness.get_by_label("item_Name_Rifle").click();
    harness.run_steps(2);
    ctrl_c(&mut harness);
    assert_eq!(copied(&harness), ["item_Name_Rifle"], "the key cell");
    Ok(())
}

#[test]
fn ctrl_c_in_the_search_field_does_not_copy_the_cell() -> TestResult {
    let (_dir, mut harness) = opened(&both(), Language::BritishEnglish)?;
    harness.get_by_label("Gewehr").click();
    harness.run_steps(2);
    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(2);

    ctrl_c(&mut harness);

    assert!(
        copied(&harness).is_empty(),
        "the field owns the shortcut: {:?}",
        copied(&harness)
    );
    Ok(())
}

#[test]
fn ctrl_c_with_selected_label_text_does_not_copy_the_cell() -> TestResult {
    let mut text = english();
    text.extend_from_slice(b"a line without an equals sign\n");
    let files = [
        (TextLanguage::English, text),
        (TextLanguage::German, localizing::german()),
    ];
    let (_dir, mut harness) = opened(&files, Language::BritishEnglish)?;
    let LocalizationStatus::Ready(table) = harness.state().localization().status() else {
        return Err("not read".into());
    };
    let note = table
        .notes()
        .first()
        .cloned()
        .ok_or("the skipped line is not noted")?;
    harness.get_by_label("Gewehr").click();
    harness.run_steps(2);
    let rect = harness.get_by_label_contains(&note).rect();
    let y = rect.center().y;

    harness.event(Event::PointerMoved(pos2(rect.left() + 1.0, y)));
    harness.step();
    harness.event(Event::PointerButton {
        pos: pos2(rect.left() + 1.0, y),
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.event(Event::PointerMoved(pos2(rect.left() + 60.0, y)));
    harness.step();
    harness.event(Event::PointerButton {
        pos: pos2(rect.left() + 60.0, y),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);
    ctrl_c(&mut harness);

    let copies = copied(&harness);
    assert!(
        copies.iter().all(|copy| copy != "Gewehr"),
        "the selected label text is what is copied: {copies:?}"
    );
    Ok(())
}

#[test]
fn a_search_that_hides_the_selected_row_clears_the_selection() -> TestResult {
    let (_dir, mut harness) = opened(&both(), Language::BritishEnglish)?;
    harness.get_by_label("Gewehr").click();
    harness.run_steps(2);
    assert!(
        harness.state().localization().selected().is_some(),
        "selected"
    );

    type_search(&mut harness, "aurora")?;

    assert!(
        harness.state().localization().selected().is_none(),
        "the row is hidden, so nothing is selected"
    );
    ctrl_c(&mut harness);
    assert!(copied(&harness).is_empty(), "nothing hidden is copied");
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Copy key")
            .accesskit_node()
            .is_disabled(),
        "the action is off"
    );
    Ok(())
}

#[test]
fn the_focused_table_scrolls_with_the_keyboard() -> TestResult {
    let mut english = GlobalIniBuilder::new();
    for index in 0..200 {
        english = english.text(format!("key_{index:05}"), format!("Text {index}"));
    }
    let files = [(TextLanguage::English, english.build())];
    let (_dir, mut harness) = opened(&files, Language::BritishEnglish)?;
    assert!(harness.query_by_label("key_00000").is_some(), "the first");
    assert!(
        harness.query_by_label("key_00199").is_none(),
        "the last is below"
    );

    harness.get_by_label("Localisation rows").focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    assert!(
        harness.query_by_label("key_00199").is_some(),
        "End shows the last"
    );
    assert!(
        harness.query_by_label("key_00000").is_none(),
        "the first is gone"
    );

    harness.key_press(Key::Home);
    harness.run();
    assert!(
        harness.query_by_label("key_00000").is_some(),
        "Home shows the first"
    );

    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(
        harness.query_by_label("key_00000").is_none(),
        "one row down"
    );
    assert!(
        harness.query_by_label("key_00001").is_some(),
        "only one row"
    );

    harness.key_press(Key::PageDown);
    harness.run();
    assert!(harness.query_by_label("key_00001").is_none(), "a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!(
        harness.query_by_label("key_00000").is_some(),
        "back at the top"
    );
    Ok(())
}

#[test]
fn the_heads_follow_another_archive_with_other_languages() -> TestResult {
    let (dir, mut harness) = opened(&both(), Language::BritishEnglish)?;
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "German")
            .is_some(),
        "German in the first"
    );
    let other = tempfile::tempdir()?;
    let path = archive(
        other.path(),
        &[
            (TextLanguage::English, english()),
            (
                TextLanguage::French,
                GlobalIniBuilder::new().text("a", "b").build(),
            ),
        ],
    )?;

    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| {
        matches!(
            model.localization().status(),
            LocalizationStatus::Ready(table) if table.languages().contains(&TextLanguage::French)
        )
    })?;

    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "French")
            .is_some(),
        "French in the second"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "German")
            .is_none(),
        "German is gone"
    );
    drop(dir);
    Ok(())
}
