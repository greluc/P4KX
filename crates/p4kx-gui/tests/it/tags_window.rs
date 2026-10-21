//! The Tags tab in the window, driven headless over a synthetic Archive with a tag tree.

use crate::tagging;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, TagsStatus, WorkTab};
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
