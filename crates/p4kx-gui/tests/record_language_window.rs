//! The language of the texts next to localisation keys in the Record preview, driven headless
//! over a synthetic DataForge and two `global.ini` files.

mod previewing_texts;

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    Browser, KeyText, LineValue, Model, PreviewTab, RecordPreviewStatus, RecordTextChoice,
    TextLanguage, ui,
};
use previewing_texts::{
    ENGLISH_AND_GERMAN, TRUCK_PATH, TestResult, archive, archive_with_broken_french,
    archive_without_texts,
};
use tempfile::TempDir;

struct Window {
    _dir: TempDir,
    harness: Harness<'static, Model>,
    settings_changed: Arc<AtomicBool>,
}

fn window(
    interface: p4kx_gui::Language,
    made: Result<(TempDir, std::path::PathBuf), Box<dyn Error>>,
) -> Result<Window, Box<dyn Error>> {
    let (dir, path) = made?;
    let settings_changed = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&settings_changed);
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 1000.0])
        .build_ui_state(
            move |ui, model: &mut Model| {
                if ui::show(ui, model).settings_changed {
                    seen.store(true, Ordering::Relaxed);
                }
            },
            Model::new(interface),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    settings_changed.store(false, Ordering::Relaxed);
    Ok(Window {
        _dir: dir,
        harness,
        settings_changed,
    })
}

fn english_window() -> Result<Window, Box<dyn Error>> {
    window(p4kx_gui::Language::BritishEnglish, archive())
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run_steps(2);
    Ok(())
}

fn preview(model: &Model) -> Option<&p4kx_gui::RecordView> {
    model
        .browser()
        .and_then(Browser::record_preview)
        .and_then(|preview| preview.view())
}

fn read_in(language: Option<TextLanguage>) -> impl Fn(&Model) -> bool {
    move |model| preview(model).is_some_and(|view| view.language() == language)
}

fn open_record(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label("Expand DataForge").click();
    wait_until(harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| tree.rows().len() > 1)
    })?;
    harness.get_by_label("Expand libs").click();
    harness.run_steps(2);
    harness.get_by_label("Expand vehicles").click();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, name).click();
    wait_until(harness, |model| preview(model).is_some())
}

fn choose(harness: &mut Harness<'_, Model>, entry: &str) {
    harness.get_by_label("Record texts").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::RadioButton, entry)
        .click();
    harness.run_steps(2);
}

fn has_row(harness: &Harness<'_, Model>, label: &str) -> bool {
    harness.query_by_role_and_label(Role::Row, label).is_some()
}

fn key_line<'v>(view: &'v p4kx_gui::RecordView, name: &str) -> Option<&'v KeyText> {
    view.lines()
        .iter()
        .find(|line| line.name() == name)
        .and_then(|line| match line.value() {
            LineValue::Key { text, .. } => Some(text),
            _ => None,
        })
}

#[test]
fn the_menu_lists_the_languages_present_in_the_archive() -> TestResult {
    let Window { mut harness, .. } = english_window()?;

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
    let Window { mut harness, .. } = english_window()?;

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
fn a_german_interface_shows_the_german_texts() -> TestResult {
    let Window { mut harness, .. } = window(p4kx_gui::Language::German, archive())?;
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
fn choosing_a_language_resolves_the_keys_and_marks_the_missing_translations() -> TestResult {
    let Window {
        mut harness,
        settings_changed,
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
        key_line(view, "[0]"),
        Some(&KeyText::Text("Karren".into())),
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
fn line_breaks_control_and_bidirectional_characters_are_shown_as_escapes() -> TestResult {
    let Window { mut harness, .. } = english_window()?;
    open_record(&mut harness, "cart")?;
    let english = preview(harness.state()).ok_or("no view")?;
    assert_eq!(
        key_line(english, "description"),
        Some(&KeyText::Text("Rolls.\\nSlowly.".into())),
        "a line break"
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
        key_line(view, "name"),
        Some(&KeyText::Text("Laster\\u{202E}\\x1Bx".into())),
        "no raw control or bidirectional character reaches the screen"
    );
    Ok(())
}

#[test]
fn switching_the_language_updates_the_open_preview_without_reading_dataforge_again() -> TestResult {
    let Window { mut harness, .. } = english_window()?;
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
    let Window { mut harness, .. } = english_window()?;
    open_record(&mut harness, "cart")?;

    harness
        .state_mut()
        .set_record_text_choice(Some(RecordTextChoice::Language(TextLanguage::German)));
    harness.step();

    let status = harness
        .state()
        .browser()
        .and_then(Browser::record_preview)
        .map(p4kx_gui::RecordPreview::status);
    assert!(
        matches!(status, Some(RecordPreviewStatus::Ready(_))),
        "the English view is still shown: {status:?}"
    );
    wait_until(&mut harness, read_in(Some(TextLanguage::German)))?;
    Ok(())
}

#[test]
fn the_xml_tab_carries_the_texts_of_the_chosen_language() -> TestResult {
    let Window { mut harness, .. } = english_window()?;
    open_record(&mut harness, "cart")?;
    choose(&mut harness, "German");
    wait_until(&mut harness, read_in(Some(TextLanguage::German)))?;

    let xml = String::from_utf8_lossy(preview(harness.state()).ok_or("no view")?.xml().bytes())
        .into_owned();

    assert!(xml.contains("text=\"Karren\""), "the German text: {xml}");
    assert!(xml.contains("missing=\"true\""), "a missing mark: {xml}");
    harness.state_mut().set_preview_tab(PreviewTab::Xml);
    harness.run_steps(3);
    Ok(())
}

#[test]
fn keys_only_leaves_the_xml_free_of_texts() -> TestResult {
    let Window { mut harness, .. } = english_window()?;
    open_record(&mut harness, "cart")?;
    choose(&mut harness, "Keys only");
    wait_until(&mut harness, read_in(None))?;

    let xml = String::from_utf8_lossy(preview(harness.state()).ok_or("no view")?.xml().bytes())
        .into_owned();

    assert!(!xml.contains("text="), "no text: {xml}");
    assert!(!xml.contains("missing="), "no missing mark: {xml}");
    Ok(())
}

#[test]
fn a_stored_language_the_archive_lacks_shows_the_keys_alone() -> TestResult {
    let Window { mut harness, .. } = english_window()?;
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
    Ok(())
}

#[test]
fn an_archive_without_texts_disables_the_menu_and_shows_the_keys() -> TestResult {
    let Window { mut harness, .. } =
        window(p4kx_gui::Language::BritishEnglish, archive_without_texts())?;
    open_record(&mut harness, "cart")?;

    assert!(harness.state().record_text_languages().is_empty(), "none");
    assert_eq!(harness.state().record_text_language(), None, "keys alone");
    assert!(has_row(&harness, "name: @vehicle_Cart"), "keys alone");
    Ok(())
}

#[test]
fn a_language_that_cannot_be_read_is_reported_and_the_keys_stay() -> TestResult {
    let Window { mut harness, .. } = window(
        p4kx_gui::Language::BritishEnglish,
        archive_with_broken_french(),
    )?;
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

#[test]
fn the_texts_of_a_german_window_are_german() -> TestResult {
    let Window { mut harness, .. } = window(p4kx_gui::Language::German, archive())?;

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
