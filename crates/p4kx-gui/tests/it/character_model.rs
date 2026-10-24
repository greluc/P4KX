// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The state of the Characters tab: the choices of body, head, pieces and Loadout, the build off
//! the UI thread and the report of what could not be fitted.

use crate::characters::{
    ARCHIVE_CHF, GRUNT, TestResult, built, chf_file, entries, opened, part_names, pick, settle,
    wait_until,
};
use crate::tagging::archive;
use p4kx_assembly::BodyVariant;
use p4kx_gui::{ChooserKind, HeadSource, Language, ViewFailure};
use p4kx_testkit::TestEntry;

#[test]
fn the_tab_builds_the_bare_male_body_when_it_opens() -> TestResult {
    let (_dir, harness) = opened(Language::BritishEnglish)?;
    let tab = harness.state().model.characters();

    assert_eq!(tab.body(), BodyVariant::Male, "the male body");
    assert_eq!(
        part_names(&harness.state().model),
        ["body"],
        "only the body"
    );
    assert!(tab.issues().is_empty(), "nothing to report");
    assert!(tab.playback().is_some(), "the skeleton plays");
    assert!(tab.camera().is_some(), "the camera frames the body");
    Ok(())
}

#[test]
fn the_archive_lists_its_chf_files_and_xml_loadouts() -> TestResult {
    let (_dir, harness) = opened(Language::BritishEnglish)?;

    let sources = harness
        .state()
        .model
        .characters()
        .sources()
        .ok_or("not listed")?;

    assert_eq!(sources.heads(), [ARCHIVE_CHF], "the CHF of the Archive");
    assert_eq!(sources.loadouts(), [GRUNT], "the Loadout without Data");
    Ok(())
}

#[test]
fn the_body_variant_is_chosen_and_rebuilds_the_character() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    let before = harness
        .state()
        .model
        .characters()
        .scene()
        .map(|s| s.serial());

    harness
        .state_mut()
        .model
        .characters_mut()
        .set_body(BodyVariant::Female);
    settle(&mut harness)?;

    let tab = harness.state().model.characters();
    assert_eq!(tab.body(), BodyVariant::Female, "the female body");
    assert_ne!(tab.scene().map(|s| s.serial()), before, "a new scene");
    assert_eq!(
        tab.playback().map(|p| p.skeleton().bone_count()),
        Some(4),
        "the female skeleton with its ponytail"
    );
    Ok(())
}

#[test]
fn a_piece_is_worn_on_the_body_and_taken_off_again() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;

    pick(&mut harness, ChooserKind::Piece, "TEST_Torso")?;
    settle(&mut harness)?;
    assert_eq!(
        part_names(&harness.state().model),
        ["body", "TEST_Torso"],
        "worn"
    );
    assert_eq!(
        harness.state().model.characters().pieces().len(),
        1,
        "listed"
    );

    harness.state_mut().model.characters_mut().remove_piece(0);
    settle(&mut harness)?;
    assert_eq!(part_names(&harness.state().model), ["body"], "taken off");
    Ok(())
}

#[test]
fn the_choice_of_a_piece_lists_only_armour_and_clothing() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;

    assert!(
        harness
            .state_mut()
            .model
            .begin_character_choice(ChooserKind::Piece)
    );

    let chooser = harness
        .state()
        .model
        .characters()
        .chooser()
        .ok_or("no choice")?;
    let names: Vec<&str> = (0..chooser.shown_len())
        .filter_map(|at| chooser.get(at))
        .map(|item| item.name())
        .collect();
    assert_eq!(
        names,
        [
            "TEST_Cuirass",
            "TEST_Helmet",
            "TEST_Pants",
            "TEST_Plate",
            "TEST_Torso"
        ],
        "worn item types only, no pistol"
    );
    Ok(())
}

#[test]
fn a_piece_the_body_cannot_wear_is_listed_in_the_report() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    harness
        .state_mut()
        .model
        .characters_mut()
        .set_body(BodyVariant::Female);
    settle(&mut harness)?;

    pick(&mut harness, ChooserKind::Piece, "TEST_Helmet")?;
    settle(&mut harness)?;

    let model = &harness.state().model;
    assert_eq!(part_names(model), ["body"], "not worn");
    let issues = model.characters().issues();
    assert_eq!(issues.len(), 1, "one entry: {issues:?}");
    assert_eq!(issues[0].name, "TEST_Helmet", "named");
    assert!(issues[0].skipped, "left out");
    Ok(())
}

#[test]
fn a_loadout_brings_its_pieces_and_a_piece_takes_the_place_of_its_type() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;

    pick(&mut harness, ChooserKind::Loadout, "Grunt.xml")?;
    settle(&mut harness)?;
    assert_eq!(
        part_names(&harness.state().model),
        ["body", "torso/TEST_Torso", "helmet/TEST_Helmet"],
        "the pieces of the Loadout"
    );
    assert_eq!(
        harness.state().model.characters().loadout(),
        Some(GRUNT),
        "named"
    );

    pick(&mut harness, ChooserKind::Piece, "TEST_Plate")?;
    settle(&mut harness)?;
    let model = &harness.state().model;
    assert_eq!(
        part_names(model),
        ["body", "helmet/TEST_Helmet", "TEST_Plate"],
        "the plate replaced the torso"
    );
    let replaced = model.characters().replaced();
    assert_eq!(replaced.len(), 1, "one replaced");
    assert_eq!(replaced[0].by(), "TEST_Plate", "by the plate");

    harness.state_mut().model.characters_mut().clear_loadout();
    settle(&mut harness)?;
    assert_eq!(
        part_names(&harness.state().model),
        ["body", "TEST_Plate"],
        "the Loadout is gone"
    );
    Ok(())
}

#[test]
fn a_head_of_the_archive_sets_the_body_and_is_worn_shaped() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;

    pick(&mut harness, ChooserKind::Head, "Runner.chf")?;
    wait_until(&mut harness, |model| {
        model.characters().head().is_some() && built(model)
    })?;

    let model = &harness.state().model;
    let head = model.characters().head().ok_or("no head")?;
    assert_eq!(head.name(), "Runner.chf", "the file");
    assert_eq!(head.variant(), BodyVariant::Male, "the body of its rig");
    assert_eq!(head.rig(), "protos_human_male_face_t1_pu", "the rig");
    assert!(
        part_names(model).contains(&"head".to_owned()),
        "the head is worn: {:?} {:?}",
        part_names(model),
        model.characters().issues()
    );
    assert!(
        model
            .characters()
            .issues()
            .iter()
            .all(|issue| !issue.reason.contains("shape")),
        "shaped by its DNA: {:?}",
        model.characters().issues()
    );

    harness
        .state_mut()
        .model
        .characters_mut()
        .set_body(BodyVariant::Female);
    assert_eq!(
        harness.state().model.characters().body(),
        BodyVariant::Male,
        "the head decides the body"
    );

    harness.state_mut().model.characters_mut().clear_head();
    settle(&mut harness)?;
    assert!(
        harness.state().model.characters().head().is_none(),
        "removed"
    );
    assert_eq!(
        part_names(&harness.state().model),
        ["body"],
        "the head is gone"
    );
    Ok(())
}

#[test]
fn a_chf_file_the_user_picks_is_read_and_a_broken_one_is_reported() -> TestResult {
    let (dir, mut harness) = opened(Language::BritishEnglish)?;
    let good = chf_file(dir.path(), "mine.chf")?;
    let broken = dir.path().join("broken.chf");
    std::fs::write(&broken, b"not a character")?;

    harness
        .state_mut()
        .model
        .characters_mut()
        .choose_head(HeadSource::File(broken));
    wait_until(&mut harness, |model| {
        model.characters().head_failure().is_some()
    })?;
    assert!(
        harness.state().model.characters().head().is_none(),
        "no head"
    );

    harness
        .state_mut()
        .model
        .characters_mut()
        .choose_head(HeadSource::File(good));
    wait_until(&mut harness, |model| {
        model.characters().head().is_some() && built(model)
    })?;
    let tab = harness.state().model.characters();
    assert!(tab.head_failure().is_none(), "the failure is cleared");
    assert_eq!(tab.head().map(|head| head.name()), Some("mine.chf"), "read");
    assert_eq!(tab.name(), "mine", "the Character is named after the file");
    Ok(())
}

#[test]
fn the_level_of_detail_is_chosen_from_the_offered_ones() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    let tab = harness.state_mut().model.characters_mut();

    tab.set_lod(9);
    assert_eq!(tab.lod(), 0, "an unknown level is ignored");
    tab.set_lod(2);
    assert_eq!(tab.lod(), 2, "a known one is taken");
    settle(&mut harness)?;
    assert_eq!(
        harness.state().model.characters().armour().map(|a| a.lod()),
        Some(2),
        "built at it"
    );
    Ok(())
}

#[test]
fn a_build_can_be_stopped_and_started_again() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    let tab = harness.state_mut().model.characters_mut();
    tab.set_lod(1);
    assert!(tab.is_building(), "building");

    tab.cancel();

    assert!(!tab.is_building(), "stopped");
    assert!(tab.is_cancelled(), "by the user");
    assert!(tab.scene().is_some(), "the view built before stays");
    tab.restart();
    settle(&mut harness)?;
    assert!(!harness.state().model.characters().is_cancelled(), "again");
    Ok(())
}

#[test]
fn a_change_of_the_archive_on_disk_clears_the_choices() -> TestResult {
    let (dir, mut harness) = opened(Language::BritishEnglish)?;
    pick(&mut harness, ChooserKind::Piece, "TEST_Torso")?;
    settle(&mut harness)?;
    let mut changed = entries();
    changed.push(TestEntry::stored("Data\\patch.txt", "patch"));
    archive(dir.path(), changed)?;

    assert!(harness.state_mut().model.check_archive(), "noticed");

    let tab = harness.state().model.characters();
    assert!(tab.pieces().is_empty(), "no pieces");
    assert!(tab.scene().is_none(), "no view");
    assert_eq!(tab.failure(), Some(&ViewFailure::ArchiveChanged), "told");
    Ok(())
}
