// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Tags tab in the window, driven headless over a synthetic Archive with a tag tree.

use crate::tagging;

use egui::accesskit::Role;
use egui::{Rect, vec2};
use egui_kittest::kittest::Queryable;
use p4kx_gui::theme::window_metrics::WORK_MIN_WIDTH;
use p4kx_gui::{Language, Model, TagsStatus, WorkTab, ui};
use p4kx_testkit::TestEntry;
use tagging::{
    MISSILE, TestResult, archive, copied, harness, opened, opened_tags, read, text, wait_until,
};

fn paths(harness: &egui_kittest::Harness<'_, Model>) -> Vec<String> {
    match harness.state().tags().status() {
        TagsStatus::Ready(view) => (0..view.shown_len())
            .filter_map(|index| view.row(index))
            .map(|row| row.tag().path().to_owned())
            .collect(),
        _ => Vec::new(),
    }
}

fn type_search(harness: &mut egui_kittest::Harness<'_, Model>, text: &str) {
    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(2);
    harness.get_by_role(Role::TextInput).type_text(text);
    harness.run_steps(3);
}

#[test]
fn the_tags_tab_sits_next_to_the_others_and_asks_for_an_archive() {
    let mut harness = harness(Model::new(Language::BritishEnglish));
    harness.run_steps(2);

    harness.get_by_role_and_label(Role::Tab, "Tags").click();
    harness.run_steps(2);

    assert_eq!(harness.state().work_tab(), WorkTab::Tags, "switched");
    assert!(
        harness.query_by_label("No Archive open").is_some(),
        "asks for an Archive"
    );
}

#[test]
fn the_tags_are_read_off_the_ui_thread_and_the_tree_starts_collapsed() -> TestResult {
    let (_dir, harness) = opened_tags(Language::BritishEnglish)?;

    assert_eq!(
        paths(&harness),
        ["Weapon", "Armour"],
        "only the roots are shown"
    );
    assert!(
        harness.query_by_label("Reading the tags…").is_none(),
        "no loading note once read"
    );
    assert!(
        harness.query_by_label("4 tags").is_some(),
        "the footer counts every tag"
    );
    for head in ["Tag", "GUID", "Legacy ID"] {
        assert!(
            harness
                .query_by_role_and_label(Role::ColumnHeader, head)
                .is_some(),
            "head {head}"
        );
    }
    Ok(())
}

#[test]
fn the_chevron_expands_and_collapses_a_tag() -> TestResult {
    let (_dir, mut harness) = opened_tags(Language::BritishEnglish)?;

    harness
        .get_by_role_and_label(Role::Button, "Expand Weapon")
        .click();
    harness.run_steps(3);

    assert_eq!(
        paths(&harness),
        ["Weapon", "Weapon.Gun", "Weapon.Missile", "Armour"],
        "the children follow their parent"
    );
    harness
        .get_by_role_and_label(Role::Button, "Collapse Weapon")
        .click();
    harness.run_steps(3);
    assert_eq!(paths(&harness), ["Weapon", "Armour"], "collapsed again");
    Ok(())
}

#[test]
fn a_search_shows_the_matches_with_the_tags_above_them() -> TestResult {
    let (_dir, mut harness) = opened_tags(Language::BritishEnglish)?;

    type_search(&mut harness, "GUN");

    assert_eq!(
        paths(&harness),
        ["Weapon", "Weapon.Gun"],
        "ignoring case, however the tree is collapsed"
    );
    assert!(harness.query_by_label("2 of 4 tags").is_some(), "the count");
    harness.get_by_role(Role::TextInput).type_text("zzz");
    harness.run_steps(3);
    assert!(
        harness
            .query_by_label("No tag path matches the search.")
            .is_some(),
        "nothing matches"
    );
    Ok(())
}

#[test]
fn a_search_shows_no_chevron_because_it_would_do_nothing() -> TestResult {
    let (_dir, mut harness) = opened_tags(Language::BritishEnglish)?;
    harness
        .get_by_role_and_label(Role::Button, "Expand Weapon")
        .click();
    harness.run_steps(3);

    type_search(&mut harness, "GUN");

    for label in ["Expand Weapon", "Collapse Weapon"] {
        assert!(
            harness
                .query_by_role_and_label(Role::Button, label)
                .is_none(),
            "no `{label}` while the search filters"
        );
    }
    if let TagsStatus::Ready(view) = harness.state().tags().status() {
        assert!(
            (0..view.shown_len())
                .filter_map(|index| view.row(index))
                .all(|row| !row.is_expandable() && !row.is_expanded()),
            "no row can be expanded or is expanded while filtered"
        );
    }
    Ok(())
}

#[test]
fn clearing_the_search_brings_the_chevrons_back_as_they_were() -> TestResult {
    let (_dir, mut harness) = opened_tags(Language::BritishEnglish)?;
    harness
        .get_by_role_and_label(Role::Button, "Expand Weapon")
        .click();
    harness.run_steps(3);
    type_search(&mut harness, "GUN");

    harness.state_mut().tags_mut().set_query("");
    harness.run_steps(3);

    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Collapse Weapon")
            .is_some(),
        "the expanded tag shows its chevron again"
    );
    Ok(())
}

#[test]
fn a_search_ignores_case_beyond_ascii() -> TestResult {
    let (_dir, mut harness) = opened(
        vec![TestEntry::zstd(
            "Data\\Game2.dcb",
            tagging::forge_named(&["Größe", "Ärmel", "Armour"]),
        )],
        Language::BritishEnglish,
    )?;

    harness.state_mut().tags_mut().set_query("GRÖSSE");
    harness.run_steps(2);
    let none = paths(&harness);
    harness.state_mut().tags_mut().set_query("ÄRMEL");
    harness.run_steps(2);
    let upper = paths(&harness);
    harness.state_mut().tags_mut().set_query("ärmel");
    harness.run_steps(2);
    let lower = paths(&harness);
    harness.state_mut().tags_mut().set_query("grö");
    harness.run_steps(2);
    let partial = paths(&harness);

    assert!(none.is_empty(), "ß is not ss: {none:?}");
    assert_eq!(upper, ["Ärmel"], "upper-case umlaut finds the tag");
    assert_eq!(lower, ["Ärmel"], "lower-case umlaut finds the tag");
    assert_eq!(partial, ["Größe"], "a prefix with an umlaut");
    Ok(())
}

#[test]
fn a_selected_tag_can_be_copied_as_path_and_as_guid() -> TestResult {
    let (_dir, mut harness) = opened_tags(Language::BritishEnglish)?;
    harness
        .get_by_role_and_label(Role::Button, "Expand Weapon")
        .click();
    harness.run_steps(3);

    harness
        .get_by_role_and_label(Role::Button, "Weapon.Missile")
        .click();
    harness.run_steps(3);
    harness
        .get_by_role_and_label(Role::Button, "Copy tag path")
        .click();
    harness.step();
    let path = copied(&harness);
    harness
        .get_by_role_and_label(Role::Button, "Copy GUID")
        .click();
    harness.step();
    let guid = copied(&harness);

    assert_eq!(path, ["Weapon.Missile"], "the tag path");
    assert_eq!(guid, [text(MISSILE)], "the GUID");
    assert_eq!(
        harness
            .state()
            .tags()
            .selected_guid()
            .map(|g| g.to_string()),
        Some(text(MISSILE))
    );
    Ok(())
}

#[test]
fn a_change_of_the_archive_on_disk_drops_the_tree_and_says_so() -> TestResult {
    let (dir, mut harness) = opened_tags(Language::BritishEnglish)?;
    archive(
        dir.path(),
        vec![TestEntry::zstd(
            "Data\\Game2.dcb",
            tagging::forge_named(&["Other"]),
        )],
    )?;

    assert!(harness.state_mut().check_archive(), "the change is noticed");
    harness.run_steps(3);

    assert!(
        matches!(harness.state().tags().status(), TagsStatus::ArchiveChanged),
        "{:?}",
        harness.state().tags().status()
    );
    assert!(
        harness
            .query_by_label("The Archive changed on disk. Reload it to read the tags.")
            .is_some(),
        "the note is shown"
    );
    Ok(())
}

#[test]
fn an_archive_without_a_dataforge_fails_with_the_reason() -> TestResult {
    let (_dir, harness) = opened(
        vec![TestEntry::stored("readme.txt", "readme")],
        Language::BritishEnglish,
    )?;

    assert!(
        matches!(harness.state().tags().status(), TagsStatus::Failed(Some(_))),
        "{:?}",
        harness.state().tags().status()
    );
    assert!(
        harness
            .query_by_label_contains("The tags cannot be read")
            .is_some(),
        "the reason is shown"
    );
    Ok(())
}

#[test]
fn a_dataforge_without_tags_says_so() -> TestResult {
    let bytes = p4kx_testkit::DataForgeBuilder::new()
        .structure(
            p4kx_testkit::TestStruct::new("Item").field("name", p4kx_testkit::TestType::String),
        )
        .build();
    let (_dir, harness) = opened(
        vec![TestEntry::zstd("Data\\Game2.dcb", bytes)],
        Language::BritishEnglish,
    )?;

    assert!(
        matches!(harness.state().tags().status(), TagsStatus::NoTags),
        "{:?}",
        harness.state().tags().status()
    );
    assert!(
        harness
            .query_by_label("The DataForge of the Archive holds no tags.")
            .is_some(),
        "the note"
    );
    Ok(())
}

#[test]
fn the_tab_is_german_in_the_german_window_and_forgets_the_tree_with_the_archive() -> TestResult {
    let (dir, mut harness) = opened_tags(Language::German)?;

    assert!(harness.query_by_label("4 Tags").is_some(), "German count");
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "Legacy-ID")
            .is_some(),
        "German head"
    );
    let path = archive(
        dir.path(),
        vec![TestEntry::zstd("Data\\Game2.dcb", tagging::forge())],
    )?;
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    assert!(
        matches!(
            harness.state().tags().status(),
            TagsStatus::Idle | TagsStatus::Loading
        ) || read(harness.state()),
        "the tree is read again for the new Archive"
    );
    Ok(())
}

#[test]
fn the_copy_buttons_lie_in_the_scrolled_work_surface_of_the_smallest_window() -> TestResult {
    for (language, guid, path) in [
        (Language::BritishEnglish, "Copy GUID", "Copy tag path"),
        (Language::German, "GUID kopieren", "Tag-Pfad kopieren"),
    ] {
        let (_dir, harness) = tagging::opened_sized(
            vec![TestEntry::zstd("Data\\Game2.dcb", tagging::forge())],
            language,
            [640.0, 400.0],
        )?;
        let work = ui::regions(&harness.ctx).work.ok_or("no work surface")?;
        let content = Rect::from_min_size(work.min, vec2(WORK_MIN_WIDTH, f32::INFINITY));
        for label in [guid, path] {
            let rect = harness.get_by_role_and_label(Role::Button, label).rect();
            assert!(
                content.contains_rect(rect),
                "{language:?}: {label} at {rect:?} lies outside the work surface content {content:?}"
            );
        }
    }
    Ok(())
}
