//! Confirmation of an Extraction from the window that covers every Entry, driven headless.

use crate::common;

use common::{TestResult, check, extract_into, files_below, finished, setup};
use p4kx_gui::{ExtractionScope, TargetKind};

#[test]
fn a_whole_archive_extraction_starts_only_after_confirmation() -> TestResult {
    let (_dir, mut model, out) = setup()?;

    let first = extract_into(
        &mut model,
        ExtractionScope::All,
        TargetKind::ExistingFolder,
        &out,
    )?;
    let confirming = model
        .extraction_setup()
        .is_some_and(p4kx_gui::ExtractionSetup::is_confirming);
    let written_before = files_below(&out)?;
    let second = model.submit_extraction(|| {});
    let summary = finished(&mut model)?;

    assert!(!first, "the first submit only asks");
    assert!(confirming, "the setup waits for confirmation");
    assert!(written_before.is_empty(), "nothing is written before");
    assert!(second, "confirming starts it");
    assert_eq!(summary.extracted(), 4, "every Entry");
    assert!(model.extraction_setup().is_none(), "the setup closes");
    Ok(())
}

#[test]
fn changing_the_scope_withdraws_the_confirmation() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    extract_into(
        &mut model,
        ExtractionScope::All,
        TargetKind::ExistingFolder,
        &out,
    )?;
    let setup = model.extraction_setup_mut().ok_or("no setup")?;

    setup.set_scope(ExtractionScope::Shown);
    setup.set_scope(ExtractionScope::All);
    let started = model.submit_extraction(|| {});

    assert!(!started, "asks again");
    assert!(model.extraction().is_none(), "nothing runs");
    Ok(())
}

#[test]
fn shown_entries_covering_the_whole_archive_ask_for_confirmation_too() -> TestResult {
    let (_dir, mut model, out) = setup()?;

    let started = extract_into(
        &mut model,
        ExtractionScope::Shown,
        TargetKind::ExistingFolder,
        &out,
    )?;

    assert!(!started, "the root shows every Entry");
    assert!(
        model
            .extraction_setup()
            .is_some_and(p4kx_gui::ExtractionSetup::is_confirming),
        "asks first"
    );
    assert!(files_below(&out)?.is_empty(), "nothing is written");
    Ok(())
}

#[test]
fn checking_every_entry_asks_for_confirmation_too() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    check(
        &mut model,
        &["config.xml", "ship.cgf", "anchor.cgf", "readme.txt"],
    )?;

    let started = extract_into(
        &mut model,
        ExtractionScope::Selected,
        TargetKind::ExistingFolder,
        &out,
    )?;

    assert!(!started, "every Entry is checked");
    assert!(model.extraction().is_none(), "nothing runs");
    Ok(())
}

#[test]
fn the_confirmation_names_the_entries_and_their_size() -> TestResult {
    let (_dir, mut model, out) = setup()?;

    extract_into(
        &mut model,
        ExtractionScope::All,
        TargetKind::ExistingFolder,
        &out,
    )?;

    let confirmation = model
        .extraction_setup()
        .and_then(p4kx_gui::ExtractionSetup::confirmation);
    assert_eq!(
        confirmation.map(|asked| (asked.entries, asked.bytes)),
        Some((4, 22)),
        "4 Entries of 6 + 4 + 6 + 6 bytes"
    );
    Ok(())
}
