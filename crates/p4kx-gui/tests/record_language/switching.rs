//! Choosing the language and switching it in the open preview.

use std::error::Error;
use std::path::Path;
use std::sync::atomic::Ordering;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_extraction::{Archive, DataForgeEntry, LanguageTexts, LocalizationSet, RecordXml};
use p4kx_gui::{
    Browser, KeyText, Model, PreviewTab, RecordPreview, RecordPreviewStatus, RecordTextChoice,
    TextLanguage,
};

use crate::fixtures::{ENGLISH_AND_GERMAN, TestResult, archive};
use crate::window::{
    Window, choose, english_window, has_row, key_line, key_text, open_record, preview, read_in,
    wait_until, window,
};

#[test]
fn the_menu_lists_the_languages_present_in_the_archive() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;

    harness.get_by_label("Record texts").click();
    harness.run_steps(2);

    for entry in ["Keys only", "English", "German"] {
        assert!(
            harness
                .query_by_role_and_label(Role::RadioButton, entry)
                .is_some(),
            "{entry}"
        );
    }
    assert!(
        harness
            .query_by_role_and_label(Role::RadioButton, "French")
            .is_none(),
        "no French file, no French entry"
    );
    assert_eq!(
        harness.state().record_text_languages(),
        ENGLISH_AND_GERMAN,
        "the model lists the same languages"
    );
    Ok(())
}

#[test]
fn the_preview_follows_the_language_of_the_interface_until_one_is_chosen() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;

    open_record(&mut harness, "cart")?;

    assert!(
        has_row(&harness, "name: @vehicle_Cart: Cart"),
        "English text next to the key"
    );
    assert_eq!(
        harness.state().record_text_language(),
        Some(TextLanguage::English),
        "the effective language"
    );
    assert_eq!(harness.state().record_text_choice(), None, "nothing chosen");
    Ok(())
}

#[test]
fn choosing_a_language_resolves_the_keys_and_marks_the_missing_translations() -> TestResult {
    let Window {
        mut harness,
        settings_changed,
        _dir,
        ..
    } = english_window()?;
    open_record(&mut harness, "cart")?;

    choose(&mut harness, "German");
    wait_until(&mut harness, read_in(Some(TextLanguage::German)))?;

    assert!(has_row(&harness, "name: @vehicle_Cart: Karren"), "German");
    assert!(
        has_row(
            &harness,
            "description: @vehicle_CartDesc (no text in this language)"
        ),
        "the description has no German text"
    );
    assert!(
        has_row(&harness, "unset: @LOC_PLACEHOLDER"),
        "a placeholder stands for no text and is not marked"
    );
    let view = preview(harness.state()).ok_or("no view")?;
    assert_eq!(
        key_text(view, "[0]"),
        Some(("Karren", &[][..])),
        "a list item"
    );
    assert_eq!(
        key_line(view, "[1]"),
        Some(&KeyText::Missing),
        "unknown key"
    );
    assert_eq!(key_line(view, "description"), Some(&KeyText::Missing));
    assert_eq!(key_line(view, "unset"), Some(&KeyText::None));
    assert_eq!(key_line(view, "empty"), Some(&KeyText::None));
    assert_eq!(
        harness.state().record_text_choice(),
        Some(RecordTextChoice::Language(TextLanguage::German)),
        "chosen"
    );
    assert!(
        settings_changed.load(Ordering::Relaxed),
        "the choice is to be stored"
    );
    Ok(())
}

#[test]
fn switching_the_language_updates_the_open_preview_without_reading_dataforge_again() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;
    open_record(&mut harness, "cart")?;
    let reads = |harness: &Harness<'_, Model>| {
        harness
            .state()
            .browser()
            .map(|browser| (browser.dataforge_reads(), browser.index().text_loads()))
    };
    assert_eq!(reads(&harness), Some((1, 1)), "DataForge and English");

    choose(&mut harness, "German");
    wait_until(&mut harness, read_in(Some(TextLanguage::German)))?;
    assert!(has_row(&harness, "name: @vehicle_Cart: Karren"), "German");
    assert_eq!(reads(&harness), Some((1, 2)), "German read, DataForge kept");

    choose(&mut harness, "English");
    wait_until(&mut harness, read_in(Some(TextLanguage::English)))?;
    assert!(has_row(&harness, "name: @vehicle_Cart: Cart"), "English");
    assert_eq!(reads(&harness), Some((1, 2)), "no file read twice");

    choose(&mut harness, "Keys only");
    wait_until(&mut harness, read_in(None))?;
    assert!(has_row(&harness, "name: @vehicle_Cart"), "keys alone");
    assert_eq!(reads(&harness), Some((1, 2)), "nothing read for keys only");
    Ok(())
}

#[test]
fn the_preview_stays_on_screen_while_the_other_language_is_read() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;
    open_record(&mut harness, "cart")?;

    harness
        .state_mut()
        .set_record_text_choice(Some(RecordTextChoice::Language(TextLanguage::German)));
    harness.step();

    let status = harness
        .state()
        .browser()
        .and_then(Browser::record_preview)
        .map(RecordPreview::status);
    assert!(
        matches!(status, Some(RecordPreviewStatus::Ready(_))),
        "the English view is still shown: {status:?}"
    );
    wait_until(&mut harness, read_in(Some(TextLanguage::German)))?;
    Ok(())
}

#[test]
fn the_xml_tab_carries_the_texts_of_the_chosen_language() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;
    open_record(&mut harness, "cart")?;
    choose(&mut harness, "German");
    wait_until(&mut harness, read_in(Some(TextLanguage::German)))?;

    let xml = String::from_utf8_lossy(preview(harness.state()).ok_or("no view")?.xml().bytes())
        .into_owned();

    assert!(xml.contains("text=\"Karren\""), "the German text: {xml}");
    assert!(xml.contains("missing=\"true\""), "a missing mark: {xml}");
    harness.state_mut().set_preview_tab(PreviewTab::Xml);
    harness.run_steps(3);

    assert!(
        harness.query_by_label_contains("text=\"Karren\"").is_some(),
        "the XML view shows the German text"
    );
    assert!(
        harness
            .query_by_label_contains("missing=\"true\"")
            .is_some(),
        "the XML view marks the missing text"
    );
    Ok(())
}

#[test]
fn the_xml_of_the_window_is_what_the_export_writes_for_the_same_language() -> TestResult {
    let (dir, path) = archive()?;
    let Window {
        mut harness, _dir, ..
    } = window(p4kx_gui::Language::BritishEnglish, Ok((dir, path.clone())))?;
    open_record(&mut harness, "cart")?;

    for language in [
        None,
        Some(TextLanguage::German),
        Some(TextLanguage::English),
    ] {
        let choice = language.map_or(RecordTextChoice::KeysOnly, RecordTextChoice::Language);
        harness.state_mut().set_record_text_choice(Some(choice));
        harness.get_by_role_and_label(Role::Button, "cart").click();
        wait_until(&mut harness, read_in(language))?;
        let view = preview(harness.state()).ok_or("no view")?;

        assert_eq!(
            view.xml().bytes(),
            exported(&path, view.guid(), language)?.as_slice(),
            "{language:?}"
        );
    }
    Ok(())
}

fn exported(
    path: &Path,
    guid: p4kx_extraction::Guid,
    language: Option<TextLanguage>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let archive = Archive::open(path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let tree = forge.record_by_guid(guid).ok_or("no such Record")?.tree()?;
    let mut out = Vec::new();
    match language {
        None => RecordXml::new().write_with(&tree, None, &mut out)?,
        Some(language) => {
            let set = LocalizationSet::read(&archive, &[language])?;
            let texts = set.index()?;
            let resolver = LanguageTexts::new(&texts, language);
            RecordXml::new().write_with(&tree, Some(&resolver), &mut out)?;
        }
    }
    Ok(out)
}

#[test]
fn keys_only_leaves_the_xml_free_of_texts() -> TestResult {
    let Window {
        mut harness, _dir, ..
    } = english_window()?;
    open_record(&mut harness, "cart")?;
    choose(&mut harness, "Keys only");
    wait_until(&mut harness, read_in(None))?;

    let xml = String::from_utf8_lossy(preview(harness.state()).ok_or("no view")?.xml().bytes())
        .into_owned();

    assert!(!xml.contains("text="), "no text: {xml}");
    assert!(!xml.contains("missing="), "no missing mark: {xml}");
    Ok(())
}
