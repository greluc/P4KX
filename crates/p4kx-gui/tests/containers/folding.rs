//! Containers behave like the folded folders of the Entry tree (ADR-0023) and like the Archive
//! for split textures: search, checks, the Shown scope and sorting cover their parts.

use p4kx_gui::{EntryRowKind, ExtractionScope, Sort, SortColumn, SortDirection};
use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};

use crate::socpak::{
    TestResult, browser, browser_mut, enter, node, opened, shown_paths, wait_until,
};

fn textured() -> TestContainer {
    TestContainer::new()
        .entry(TestEntry::stored("Tex\\wall.dds", vec![4; 100]))
        .entry(TestEntry::stored("tex/wall.dds.1", vec![5; 20]))
        .entry(TestEntry::stored("Tex\\wall.dds.2", vec![6; 3]))
        .entry(TestEntry::stored("Tex\\lonely.dds", vec![2; 50]))
        .entry(TestEntry::stored("Props\\a.xml", "a"))
        .entry(TestEntry::stored("props/b.xml", "b"))
}

fn archive() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Objects\\Room.socpak",
            textured().build(),
        ))
        .entry(TestEntry::stored(
            "data\\objects\\alpha.socpak",
            TestContainer::new().build(),
        ))
        .entry(TestEntry::stored(
            "Data/Objects/zeta.socpak",
            TestContainer::new().build(),
        ))
}

fn search(model: &mut p4kx_gui::Model, text: &str) -> TestResult {
    browser_mut(model)?.search(text, || {});
    wait_until(model, |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching())
    })
}

#[test]
fn containers_in_folders_that_differ_by_case_or_separator_share_one_folder() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    for name in ["data", "objects"] {
        let folder = node(&model, name)?;
        browser_mut(&mut model)?.toggle_expanded(folder);
    }

    let browser = browser(&model)?;
    let names: Vec<&str> = browser
        .tree_rows()
        .iter()
        .skip(3)
        .map(|row| browser.node_name(row.node))
        .collect();
    assert_eq!(
        names,
        ["alpha.socpak", "Room.socpak", "zeta.socpak"],
        "one folder holds all three, by name ignoring case"
    );
    Ok(())
}

#[test]
fn folders_inside_a_container_fold_case_and_separators() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    for name in ["data", "objects"] {
        let folder = node(&model, name)?;
        browser_mut(&mut model)?.toggle_expanded(folder);
    }
    crate::socpak::expand(&mut model, "Room.socpak")?;

    let browser = browser(&model)?;
    let names: Vec<&str> = browser
        .tree_rows()
        .iter()
        .skip(5)
        .take(2)
        .map(|row| browser.node_name(row.node))
        .collect();
    assert_eq!(names, ["Props", "Tex"], "Props and props are one folder");
    Ok(())
}

#[test]
fn a_split_texture_in_a_container_is_one_row_that_checks_and_counts_its_parts() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    for name in ["data", "objects"] {
        let folder = node(&model, name)?;
        browser_mut(&mut model)?.toggle_expanded(folder);
    }
    enter(&mut model, "Room.socpak")?;

    let browser = browser_mut(&mut model)?;
    let kinds: Vec<_> = browser.rows().iter().map(|row| row.kind).collect();
    assert!(
        kinds.contains(&EntryRowKind::Texture { parts: 2 }),
        "the texture counts its parts: {kinds:?}"
    );
    assert_eq!(browser.view_len(), 6, "the rows stand for six Entries");
    assert_eq!(browser.scope_len(ExtractionScope::Shown), 6, "Shown scope");
    assert_eq!(
        browser.scope_entries(ExtractionScope::Shown).len(),
        6,
        "Shown Entries include the parts"
    );
    let header = browser
        .shown()
        .iter()
        .copied()
        .find(|&id| browser.index().texture_parts(id).len() == 2)
        .ok_or("no texture header")?;
    browser.set_checked(header, true);
    assert_eq!(browser.checked_count(), 3, "header and both parts");
    assert_eq!(
        browser.scope_len(ExtractionScope::Selected),
        3,
        "Selected scope"
    );
    browser.set_checked(header, false);
    assert_eq!(browser.checked_count(), 0, "all parts unchecked together");
    Ok(())
}

#[test]
fn search_and_sorting_inside_a_container_keep_a_texture_together() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    for name in ["data", "objects"] {
        let folder = node(&model, name)?;
        browser_mut(&mut model)?.toggle_expanded(folder);
    }
    enter(&mut model, "Room.socpak")?;

    search(&mut model, "wall")?;
    let browser = browser(&model)?;
    assert_eq!(browser.view_len(), 3, "a match of one part shows all three");
    assert_eq!(browser.rows().len(), 1, "one row for the texture");
    assert_eq!(shown_paths(&model)?, ["Tex\\wall.dds"], "the header");

    browser_mut(&mut model)?.sort_by(SortColumn::Size);
    assert_eq!(
        browser_mut(&mut model)?.sort(),
        Sort {
            column: SortColumn::Size,
            direction: SortDirection::Ascending
        },
        "sorted by size"
    );
    Ok(())
}
