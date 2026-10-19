//! Interface languages, hostile texts and Archives without usable texts.

use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{Browser, Language, Model, RecordTextChoice, TextLanguage};

use crate::fixtures::{
    TRUCK_PATH, TestResult, archive, archive_with_broken_french, archive_without_texts,
};
use crate::window::{
    Window, choose, english_window, has_row, key_text, open_record, preview, read_in, wait_until,
    window,
};

#[test]
fn a_german_interface_shows_the_german_texts() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = window(Language::German, archive())?;
    harness.get_by_label("DataForge aufklappen").click();
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| tree.rows().len() > 1)
    })?;
    harness.get_by_label("libs aufklappen").click();
    harness.run_steps(2);
    harness.get_by_label("vehicles aufklappen").click();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, "cart").click();
    wait_until(&mut harness, |model| preview(model).is_some())?;

    assert!(
        has_row(&harness, "name: @vehicle_Cart: Karren"),
        "German text next to the key"
    );
    Ok(())
}

#[test]
fn the_menu_of_a_german_window_is_german() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = window(Language::German, archive())?;

    harness.get_by_label("Record-Texte").click();
    harness.run_steps(2);

    for entry in ["Nur Schlüssel", "Englisch", "Deutsch"] {
        assert!(
            harness
                .query_by_role_and_label(Role::RadioButton, entry)
                .is_some(),
            "{entry}"
        );
    }
    Ok(())
}

#[test]
fn line_breaks_control_and_bidirectional_characters_are_shown_as_escapes() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;
    open_record(&mut harness, "cart")?;
    let english = preview(harness.state()).ok_or("no view")?;
    assert_eq!(
        key_text(english, "description"),
        Some(("Rolls.\\nSlowly.", std::slice::from_ref(&(6..8)))),
        "a line break is an escape"
    );
    harness
        .state_mut()
        .set_record_text_choice(Some(RecordTextChoice::Language(TextLanguage::German)));
    harness.get_by_role_and_label(Role::Button, "truck").click();
    wait_until(&mut harness, |model| {
        read_in(Some(TextLanguage::German))(model)
            && preview(model).is_some_and(|view| view.record_path() == TRUCK_PATH)
    })?;

    let view = preview(harness.state()).ok_or("no view")?;
    assert_eq!(
        key_text(view, "name"),
        Some(("Laster\\u{202E}\\x1Bx", &[6..14, 14..18][..])),
        "no raw control or bidirectional character reaches the screen"
    );
    Ok(())
}

#[test]
fn a_stored_language_the_archive_lacks_shows_the_keys_alone() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;
    harness
        .state_mut()
        .set_record_text_choice(Some(RecordTextChoice::Language(TextLanguage::French)));

    assert_eq!(
        harness.state().record_text_language(),
        None,
        "no French file"
    );
    open_record(&mut harness, "cart")?;
    assert!(has_row(&harness, "name: @vehicle_Cart"), "keys alone");

    harness.get_by_label("Record texts").click();
    harness.run_steps(2);
    let selected = |harness: &Harness<'_, Model>, entry: &str| {
        harness
            .get_by_role_and_label(Role::RadioButton, entry)
            .accesskit_node()
            .toggled()
            == Some(Toggled::True)
    };
    assert!(
        !selected(&harness, "Keys only"),
        "the user never chose to see the keys alone"
    );
    assert!(!selected(&harness, "English") && !selected(&harness, "German"));
    assert!(
        harness
            .query_by_label_contains("holds no texts in French")
            .is_some(),
        "the menu says why only the keys are shown"
    );
    Ok(())
}

#[test]
fn keys_only_chosen_stays_selected_and_needs_no_note() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;
    choose(&mut harness, "Keys only");
    harness.get_by_label("Record texts").click();
    harness.run_steps(2);

    assert_eq!(
        harness
            .get_by_role_and_label(Role::RadioButton, "Keys only")
            .accesskit_node()
            .toggled(),
        Some(Toggled::True)
    );
    assert!(harness.query_by_label_contains("holds no texts").is_none());
    Ok(())
}

#[test]
fn a_menu_without_languages_is_reported_as_disabled() -> TestResult {
    let Window { harness, _dir, .. } = window(Language::BritishEnglish, archive_without_texts())?;

    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Record texts")
            .accesskit_node()
            .is_disabled(),
        "assistive technology hears a disabled menu"
    );
    Ok(())
}

#[test]
fn an_archive_without_texts_shows_the_keys_and_offers_no_language() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = window(Language::BritishEnglish, archive_without_texts())?;
    open_record(&mut harness, "cart")?;

    assert!(harness.state().record_text_languages().is_empty(), "none");
    assert_eq!(harness.state().record_text_language(), None, "keys alone");
    assert!(has_row(&harness, "name: @vehicle_Cart"), "keys alone");
    Ok(())
}

#[test]
fn a_language_that_cannot_be_read_is_reported_and_the_keys_stay() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = window(Language::BritishEnglish, archive_with_broken_french())?;
    open_record(&mut harness, "cart")?;
    assert!(
        has_row(&harness, "name: @vehicle_Cart: Cart"),
        "English works"
    );

    choose(&mut harness, "French");
    wait_until(&mut harness, |model| {
        preview(model).is_some_and(|view| view.texts_failure().is_some())
    })?;

    assert!(
        harness
            .query_by_label_contains(
                "The texts in French cannot be read, so only the keys are shown"
            )
            .is_some(),
        "the alert"
    );
    assert!(has_row(&harness, "name: @vehicle_Cart"), "the keys stay");
    let view = preview(harness.state()).ok_or("no view")?;
    assert_eq!(view.language(), None, "no language resolved");
    Ok(())
}
