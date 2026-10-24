// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Archive key dialog: it opens when an encrypted Entry is needed, checks and stores the key,
//! and the window reads the Entry once it fits.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, KeyProblem, KeyReason, Language, Model, PreviewStatus, ui};
use p4kx_settings::KeyStore;
use p4kx_testkit::{ArchiveBuilder, TestEntry, other_test_key, test_key};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;
type Window = Harness<'static, Model>;

struct Setup {
    _folder: TempDir,
    store: PathBuf,
    archive: PathBuf,
}

fn setup() -> Result<Setup, Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let archive = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\plain.txt", "plain text"))
        .entry(TestEntry::stored("Data\\secret.txt", "secret text").encrypted())
        .entry(TestEntry::deflated("Data\\other.txt", "other secret").encrypted())
        .write_to(&archive)?;
    let store = folder.path().join("profile").join("archive.key");
    Ok(Setup {
        _folder: folder,
        store,
        archive,
    })
}

fn window(
    language: Language,
    setup: &Setup,
    stored: Option<&str>,
) -> Result<Window, Box<dyn Error>> {
    if let Some(hex) = stored {
        let key = p4kx_gui::ArchiveKey::from_hex(hex)?;
        KeyStore::new(&setup.store).save(&key)?;
    }
    let store = KeyStore::new(&setup.store);
    let in_use = store.load()?;
    keyless_window(language, setup, |model| {
        model.set_key_store(Some(store));
        model.set_archive_key(in_use);
    })
}

fn wait_until(harness: &mut Window, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}

fn select(harness: &mut Window, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })
}

fn status(harness: &Window) -> String {
    harness
        .state()
        .browser()
        .and_then(Browser::preview)
        .map_or_else(String::new, |preview| match preview.status() {
            PreviewStatus::KeyRequired => "required".to_owned(),
            PreviewStatus::KeyRejected => "rejected".to_owned(),
            PreviewStatus::Ready(_) => "ready".to_owned(),
            other => format!("{other:?}"),
        })
}

fn type_key(harness: &mut Window, text: &str) {
    if let Some(dialog) = harness.state_mut().key_dialog_mut() {
        dialog.set_text(text.to_owned());
    }
    harness.run();
}

#[test]
fn an_encrypted_entry_without_a_key_opens_the_dialog_and_says_so_in_the_preview() -> TestResult {
    let setup = setup()?;
    let mut harness = window(Language::BritishEnglish, &setup, None)?;
    assert!(harness.state().key_dialog().is_none(), "closed at first");

    select(&mut harness, "plain.txt")?;
    assert!(
        harness.state().key_dialog().is_none(),
        "plain Entry needs none"
    );
    select(&mut harness, "secret.txt")?;
    harness.run();

    assert_eq!(status(&harness), "required", "preview");
    assert_eq!(
        harness
            .state()
            .key_dialog()
            .map(p4kx_gui::KeyDialog::reason),
        Some(KeyReason::Needed),
        "dialog"
    );
    assert!(
        harness
            .query_by_label_contains("is encrypted. Enter the Archive key")
            .is_some(),
        "preview note"
    );
    assert!(harness.query_by_label("KEY").is_some(), "key field");
    Ok(())
}

#[test]
fn the_dialog_names_what_is_wrong_with_the_key_typed() -> TestResult {
    let setup = setup()?;
    let mut harness = window(Language::BritishEnglish, &setup, None)?;
    select(&mut harness, "secret.txt")?;
    harness.run();

    type_key(&mut harness, "abcd");
    harness.get_by_label("Save").click();
    harness.run();
    assert!(
        harness
            .query_by_label("A key has 32 hexadecimal digits; you entered 4.")
            .is_some(),
        "length"
    );

    type_key(&mut harness, &"g".repeat(32));
    harness.get_by_label("Save").click();
    harness.run();
    assert!(
        harness
            .query_by_label("Character 1 is not a hexadecimal digit.")
            .is_some(),
        "character"
    );

    type_key(&mut harness, &other_test_key().to_hex());
    harness.get_by_label("Save").click();
    wait_until(&mut harness, |model| {
        model
            .key_dialog()
            .is_some_and(|dialog| !dialog.pending() && dialog.problem().is_some())
    })?;
    assert_eq!(
        harness
            .state()
            .key_dialog()
            .and_then(|dialog| dialog.problem().cloned()),
        Some(KeyProblem::Wrong),
        "wrong key"
    );
    assert!(
        harness
            .query_by_label("This key does not decrypt the Archive.")
            .is_some(),
        "wrong key text"
    );
    assert!(!setup.store.exists(), "nothing stored for a wrong key");
    Ok(())
}

#[test]
fn a_right_key_is_stored_used_and_the_entry_is_read_again() -> TestResult {
    let setup = setup()?;
    let mut harness = window(Language::BritishEnglish, &setup, None)?;
    select(&mut harness, "secret.txt")?;
    harness.run();

    type_key(&mut harness, &format!("  {}  ", test_key().to_hex()));
    harness.get_by_label("Save").click();
    wait_until(&mut harness, |model| {
        model.key_dialog().is_none()
            && model
                .browser()
                .and_then(Browser::preview)
                .is_some_and(|preview| preview.contents().is_some())
    })?;

    assert_eq!(status(&harness), "ready", "preview");
    assert!(
        harness.query_by_label("secret text").is_some(),
        "decrypted text"
    );
    assert_eq!(
        KeyStore::new(&setup.store).load()?,
        Some(test_key()),
        "stored in the profile"
    );
    select(&mut harness, "other.txt")?;
    assert_eq!(status(&harness), "ready", "another encrypted Entry");
    assert!(harness.state().key_dialog().is_none(), "no second dialog");
    Ok(())
}

#[test]
fn a_dialog_closed_by_the_user_stays_closed_until_asked_for_in_the_menu() -> TestResult {
    let setup = setup()?;
    let mut harness = window(Language::BritishEnglish, &setup, None)?;
    select(&mut harness, "secret.txt")?;
    harness.run();
    assert!(harness.state().key_dialog().is_some(), "opened");

    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(harness.state().key_dialog().is_none(), "closed");

    select(&mut harness, "other.txt")?;
    harness.run();
    assert!(harness.state().key_dialog().is_none(), "not asked again");
    assert_eq!(status(&harness), "required", "preview still says so");

    harness.get_by_label("Archive").click();
    harness.run();
    harness.get_by_label("Archive key…").click();
    harness.run();
    assert_eq!(
        harness
            .state()
            .key_dialog()
            .map(p4kx_gui::KeyDialog::reason),
        Some(KeyReason::Menu),
        "opened from the menu"
    );
    Ok(())
}

#[test]
fn a_stored_key_that_does_not_fit_opens_the_dialog_as_rejected() -> TestResult {
    let setup = setup()?;
    let mut harness = window(
        Language::BritishEnglish,
        &setup,
        Some(&other_test_key().to_hex()),
    )?;

    select(&mut harness, "secret.txt")?;
    harness.run();

    assert_eq!(status(&harness), "rejected", "preview");
    assert_eq!(
        harness
            .state()
            .key_dialog()
            .map(p4kx_gui::KeyDialog::reason),
        Some(KeyReason::Rejected),
        "dialog"
    );
    assert!(
        harness
            .query_by_label_contains("The key in use does not decrypt this Archive")
            .is_some(),
        "reason"
    );
    Ok(())
}

#[test]
fn the_stored_key_is_shown_masked_and_can_be_removed() -> TestResult {
    let setup = setup()?;
    let hex = test_key().to_hex();
    let mut harness = window(Language::BritishEnglish, &setup, Some(&hex))?;
    harness.state_mut().open_key_dialog(KeyReason::Menu);
    harness.run();

    let masked = test_key().masked();
    assert!(
        harness
            .query_by_label(&format!("A key is stored: {masked}"))
            .is_some(),
        "masked"
    );
    assert!(
        harness.query_by_label_contains(&hex).is_none(),
        "never in full"
    );

    harness.get_by_label("Remove stored key").click();
    harness.run();

    assert!(!setup.store.exists(), "file removed");
    assert!(!harness.state().has_archive_key(), "no key in use");
    assert!(
        harness.query_by_label("No key is stored.").is_some(),
        "note"
    );
    Ok(())
}

#[test]
fn the_dialog_speaks_german_too() -> TestResult {
    let setup = setup()?;
    let mut harness = window(Language::German, &setup, None)?;
    select(&mut harness, "secret.txt")?;
    harness.run();

    for text in [
        "SCHLÜSSEL",
        "Speichern",
        "Abbrechen",
        "Es ist kein Schlüssel gespeichert.",
    ] {
        assert!(harness.query_by_label(text).is_some(), "{text}");
    }
    Ok(())
}

fn keyless_window(
    language: Language,
    setup: &Setup,
    prepare: impl FnOnce(&mut Model),
) -> Result<Window, Box<dyn Error>> {
    let mut model = Model::new(language);
    prepare(&mut model);
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1200.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(setup.archive.clone(), || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok(harness)
}

mod notes;
