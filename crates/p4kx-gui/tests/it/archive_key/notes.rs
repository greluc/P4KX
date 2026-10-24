// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Archive key dialog: what it says about a malformed key source and how it checks a key.

use egui_kittest::kittest::Queryable;
use p4kx_gui::{KeyReason, Language};
use p4kx_settings::KeyStore;
use p4kx_testkit::test_key;

use super::{TestResult, keyless_window, setup, type_key, wait_until, window};

#[test]
fn a_malformed_variable_or_stored_key_is_named_in_the_dialog() -> TestResult {
    let setup = setup()?;
    let folder = setup.store.parent().ok_or("a folder")?;
    std::fs::create_dir_all(folder)?;
    std::fs::write(&setup.store, "this is not a key\n")?;
    let mut harness = keyless_window(Language::BritishEnglish, &setup, |model| {
        model.set_key_store(Some(KeyStore::new(&setup.store)));
        model.set_key_variable_invalid(true);
    })?;

    harness.state_mut().open_key_dialog(KeyReason::Menu);
    harness.run();

    assert!(
        harness
            .query_by_label_contains("environment variable P4KX_ARCHIVE_KEY")
            .is_some(),
        "variable"
    );
    assert!(
        harness
            .query_by_label_contains("The stored key cannot be used")
            .is_some(),
        "stored key"
    );
    assert!(
        harness
            .query_by_label_contains("this is not a key")
            .is_none(),
        "the content is not shown"
    );
    Ok(())
}

#[test]
fn the_key_is_checked_on_a_worker_and_the_dialog_is_pending_meanwhile() -> TestResult {
    let setup = setup()?;
    let mut harness = window(Language::BritishEnglish, &setup, None)?;
    harness.state_mut().open_key_dialog(KeyReason::Menu);
    type_key(&mut harness, &test_key().to_hex());

    let started = harness.state_mut().submit_key_dialog(|| {});

    assert!(started, "the check started");
    assert!(
        harness
            .state()
            .key_dialog()
            .is_some_and(p4kx_gui::KeyDialog::pending),
        "pending until the window takes the answer"
    );
    assert!(
        !harness.state_mut().submit_key_dialog(|| {}),
        "no second check while one runs"
    );
    wait_until(&mut harness, |model| model.key_dialog().is_none())?;
    assert!(harness.state().has_archive_key(), "key taken");
    Ok(())
}
