// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The localisation tab in the window, driven headless over a synthetic Archive with two
//! `global.ini` files.

use crate::localizing;

use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};
use localizing::{
    TestResult, archive, both, copied, english, harness, opened, ready, type_search, wait_until,
};
use p4kx_gui::{Language, LocalizationStatus, Model, TextLanguage, WorkTab};
use p4kx_testkit::{ArchiveBuilder, GlobalIniBuilder, TestEntry};

const FRAME_BUDGET: Duration = Duration::from_millis(500);
const FRAME_AND_SEARCH_BUDGET: Duration = Duration::from_secs(10);

#[test]
fn the_localisation_tab_sits_next_to_the_others_and_asks_for_an_archive() {
    let mut harness = harness(Model::new(Language::BritishEnglish));
    harness.run_steps(2);

    harness
        .get_by_role_and_label(Role::Tab, "Localisation")
        .click();
    harness.run_steps(2);

    assert_eq!(
        harness.state().work_tab(),
        WorkTab::Localization,
        "switched"
    );
    assert!(
        harness.query_by_label("No Archive open").is_some(),
        "asks for an Archive"
    );
}

#[test]
fn the_files_are_read_off_the_ui_thread_behind_a_loading_indicator() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), &both())?;
    let mut harness = harness(Model::new(Language::BritishEnglish));
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Localization);

    harness.step();

    assert!(
        matches!(
            harness.state().localization().status(),
            LocalizationStatus::Loading | LocalizationStatus::Ready(_)
        ),
        "started by showing the tab"
    );
    if matches!(
        harness.state().localization().status(),
        LocalizationStatus::Loading
    ) {
        assert!(
            harness
                .query_by_label("Reading the localisation files…")
                .is_some(),
            "loading text"
        );
    }
    wait_until(&mut harness, ready)?;
    assert!(
        harness
            .query_by_label("Reading the localisation files…")
            .is_none(),
        "gone once read"
    );
    Ok(())
}

#[test]
fn the_table_has_one_column_per_language_and_one_row_per_key() -> TestResult {
    let (_dir, harness) = opened(&both(), Language::BritishEnglish)?;

    for head in ["Key", "English", "German"] {
        assert!(
            harness
                .query_by_role_and_label(Role::ColumnHeader, head)
                .is_some(),
            "head {head}"
        );
    }
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "French")
            .is_none(),
        "no column for a language the Archive lacks"
    );
    for cell in [
        "item_Name_Rifle",
        "Rifle",
        "Gewehr",
        "vehicle_Name_Aurora",
        "only_english",
    ] {
        assert!(harness.query_by_label(cell).is_some(), "cell {cell}");
    }
    assert!(harness.query_by_label("4 keys").is_some(), "count");
    let model = &harness.state();
    assert_eq!(model.localization().shown_len(), 4, "four keys, once each");
    Ok(())
}

#[test]
fn a_missing_translation_is_marked() -> TestResult {
    let (_dir, harness) = opened(&both(), Language::BritishEnglish)?;

    let marks = harness
        .query_all_by_role_and_label(Role::Cell, "Missing")
        .count();

    assert_eq!(marks, 1, "only_english has no German text");
    Ok(())
}

#[test]
fn the_search_matches_keys_and_texts_ignoring_case_while_typing() -> TestResult {
    let (_dir, mut harness) = opened(&both(), Language::BritishEnglish)?;

    type_search(&mut harness, "RIFLE")?;
    assert_eq!(
        harness.state().localization().shown_len(),
        2,
        "the key of two rows"
    );
    assert!(harness.query_by_label("2 of 4 keys").is_some(), "count");

    harness.get_by_role(Role::TextInput).type_text("x");
    type_search(&mut harness, "")?;
    assert_eq!(
        harness.state().localization().shown_len(),
        0,
        "no key or text contains RIFLEx"
    );
    assert!(
        harness
            .query_by_label("No key or text matches the search.")
            .is_some(),
        "explained"
    );
    Ok(())
}

#[test]
fn the_search_finds_a_text_in_any_language() -> TestResult {
    let (_dir, mut harness) = opened(&both(), Language::BritishEnglish)?;

    type_search(&mut harness, "gewehr")?;

    assert_eq!(harness.state().localization().shown_len(), 1, "one row");
    assert!(harness.query_by_label("Gewehr").is_some(), "the row");
    assert!(
        harness.query_by_label("only_english").is_none(),
        "others hidden"
    );
    Ok(())
}

#[test]
fn a_leading_at_sign_of_a_pasted_key_is_ignored() -> TestResult {
    let (_dir, mut harness) = opened(&both(), Language::BritishEnglish)?;

    type_search(&mut harness, "@vehicle_name")?;

    assert_eq!(harness.state().localization().shown_len(), 1, "one row");
    Ok(())
}

#[test]
fn the_key_and_the_text_of_a_row_can_be_copied() -> TestResult {
    let (_dir, mut harness) = opened(&both(), Language::BritishEnglish)?;
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Copy key")
            .accesskit_node()
            .is_disabled(),
        "nothing selected yet"
    );

    harness.get_by_label("Gewehr").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "Copy key")
        .click();
    harness.step();
    assert_eq!(
        copied(&harness),
        ["item_Name_Rifle"],
        "the key as the first language spells it"
    );

    harness.get_by_label("Schießt.\\nGut.").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "Copy text")
        .click();
    harness.step();
    assert_eq!(
        copied(&harness),
        ["Schießt.\nGut."],
        "the text with its line break decoded"
    );
    Ok(())
}

#[test]
fn control_and_bidirectional_characters_are_shown_as_escapes() -> TestResult {
    let (_dir, harness) = opened(&both(), Language::BritishEnglish)?;

    assert!(
        harness
            .query_by_label("Aurora MR \\u{202E}evil\\x1B")
            .is_some(),
        "escaped"
    );
    Ok(())
}

#[test]
fn the_german_tab_uses_german_texts() -> TestResult {
    let (_dir, harness) = opened(&both(), Language::German)?;

    assert!(
        harness
            .query_by_role_and_label(Role::Tab, "Lokalisierung")
            .is_some(),
        "tab"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "Schlüssel")
            .is_some(),
        "key head"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "Englisch")
            .is_some(),
        "language head"
    );
    assert!(harness.query_by_label("4 Schlüssel").is_some(), "count");
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Cell, "Fehlt")
            .count(),
        1,
        "mark"
    );
    Ok(())
}

#[test]
fn an_archive_without_a_global_ini_says_so() -> TestResult {
    let (_dir, harness) = opened(&[], Language::BritishEnglish)?;

    assert!(
        matches!(
            harness.state().localization().status(),
            LocalizationStatus::NoTexts
        ),
        "no texts"
    );
    assert!(
        harness
            .query_by_label("The Archive holds no global.ini.")
            .is_some(),
        "explained"
    );
    Ok(())
}

#[test]
fn a_global_ini_that_cannot_be_read_is_reported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let text = english();
    let claimed = u64::try_from(text.len())? + 16;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(
            TestEntry::stored(TextLanguage::English.global_ini_path(), text)
                .with_uncompressed_size(claimed),
        )
        .write_to(&path)?;
    let mut harness = harness(Model::new(Language::BritishEnglish));
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Localization);
    harness.run_steps(2);

    wait_until(&mut harness, ready)?;

    assert!(
        matches!(
            harness.state().localization().status(),
            LocalizationStatus::Failed(Some(_))
        ),
        "failed"
    );
    assert!(
        harness
            .query_by_label_contains("The localisation files cannot be read")
            .is_some(),
        "reason shown"
    );
    Ok(())
}

#[test]
fn a_big_table_is_searched_without_blocking_the_window() -> TestResult {
    let mut english = GlobalIniBuilder::new();
    let mut german = GlobalIniBuilder::new();
    for index in 0..40_000 {
        english = english.text(format!("key_{index:05}"), format!("Text number {index}"));
        german = german.text(format!("key_{index:05}"), format!("Text Nummer {index}"));
    }
    let files = [
        (TextLanguage::English, english.build()),
        (TextLanguage::German, german.build()),
    ];
    let (_dir, mut harness) = opened(&files, Language::BritishEnglish)?;
    assert_eq!(
        harness.state().localization().shown_len(),
        40_000,
        "all keys"
    );

    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(2);
    harness
        .get_by_role(Role::TextInput)
        .type_text("nummer 3999");
    let started = Instant::now();
    let mut slowest = Duration::ZERO;
    loop {
        let frame = Instant::now();
        harness.step();
        slowest = slowest.max(frame.elapsed());
        if !harness.state().localization().is_searching() {
            break;
        }
        if started.elapsed() > FRAME_AND_SEARCH_BUDGET {
            return Err("the search did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    wait_until(&mut harness, |model| !model.localization().is_searching())?;

    assert_eq!(
        harness.state().localization().shown_len(),
        11,
        "3999 and 39990 to 39999"
    );
    assert!(
        slowest < FRAME_BUDGET,
        "a frame took {slowest:?} while the table was searched"
    );
    Ok(())
}
