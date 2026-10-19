//! Containers as folders of the path tree: opened off the UI thread, browsed, kept and dropped
//! again.

use p4kx_gui::{ContainerFailure, ContainerStatus, Place, TreeNode};

use crate::socpak::{
    self, TestResult, archive, browser, browser_mut, container, enter, expand, node, opened,
    show_containers, shown_paths, wait_until,
};

#[test]
fn containers_follow_the_subfolders_of_their_folder_by_name() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;

    show_containers(&mut model)?;

    let browser = browser(&model)?;
    let names: Vec<&str> = browser
        .tree_rows()
        .iter()
        .skip(2)
        .map(|row| browser.node_name(row.node))
        .collect();
    assert_eq!(
        names,
        [
            "Libs",
            "ObjectContainers",
            "broken.socpak",
            "packed.socpak",
            "room.socpak"
        ],
        "the containers below their folder by name"
    );
    let id = container(&model, "room.socpak")?;
    assert!(
        matches!(browser.container_status(id), ContainerStatus::Closed),
        "nothing is opened before it is expanded"
    );
    assert!(browser.is_expandable(TreeNode::container(id)), "chevron");
    Ok(())
}

#[test]
fn expanding_a_container_opens_it_and_shows_its_folders() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let id = container(&model, "room.socpak")?;

    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    let opening = matches!(
        browser(&model)?.container_status(id),
        ContainerStatus::Opening
    );
    wait_until(&mut model, |model| socpak::settled(model, id))?;

    assert!(opening, "it opens on a background thread");
    assert!(
        matches!(browser(&model)?.container_status(id), ContainerStatus::Open),
        "open"
    );
    let room = node(&model, "room")?;
    assert_eq!(
        room.place,
        Place::Container(id),
        "a folder of the container"
    );
    browser_mut(&mut model)?.toggle_expanded(room);
    assert!(node(&model, "props").is_ok(), "folders inside");
    assert!(node(&model, "lights.socpak").is_ok(), "a nested container");
    assert_eq!(
        browser(&model)?.place(),
        Place::Archive,
        "expanding does not change the shown Entries"
    );
    Ok(())
}

#[test]
fn nested_containers_expand_too() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    expand(&mut model, "room.socpak")?;
    let room = node(&model, "room")?;
    browser_mut(&mut model)?.toggle_expanded(room);

    let lights = enter(&mut model, "lights.socpak")?;

    assert!(
        matches!(
            browser(&model)?.container_status(lights),
            ContainerStatus::Open
        ),
        "open"
    );
    assert_eq!(shown_paths(&model)?, ["lamp.xml"], "its Entries");
    Ok(())
}

#[test]
fn selecting_a_container_shows_its_entries() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;

    enter(&mut model, "room.socpak")?;

    assert_eq!(
        shown_paths(&model)?,
        [
            "room\\props\\chair.cgf",
            "room\\layout.xml",
            "room\\lights.socpak",
            "room\\props\\table.cgf",
        ],
        "every Entry of the container by name"
    );
    let browser = browser(&model)?;
    assert_eq!(browser.index().len(), 4, "the index of the container");
    assert_eq!(browser.archive_index().len(), 5, "the Archive stays open");
    Ok(())
}

#[test]
fn a_folder_inside_a_container_shows_its_entries() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    expand(&mut model, "room.socpak")?;
    let room = node(&model, "room")?;
    browser_mut(&mut model)?.toggle_expanded(room);

    let props = node(&model, "props")?;
    browser_mut(&mut model)?.select_folder(props);

    assert_eq!(
        shown_paths(&model)?,
        ["room\\props\\chair.cgf", "room\\props\\table.cgf"],
        "the folder only"
    );
    Ok(())
}

#[test]
fn a_compressed_container_is_decoded_once() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let id = expand(&mut model, "packed.socpak")?;
    let first = browser(&model)?
        .place_index(Place::Container(id))
        .map(std::ptr::from_ref);
    let used = browser(&model)?.containers_used();

    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    let again = browser(&model)?
        .place_index(Place::Container(id))
        .map(std::ptr::from_ref);

    assert!(first.is_some(), "open");
    assert_eq!(first, again, "the cached container, not a new one");
    assert!(used > 4096, "its decoded data counts against the budget");
    Ok(())
}

#[test]
fn a_corrupt_container_shows_why_on_its_node() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;

    let id = expand(&mut model, "broken.socpak")?;

    let browser = browser(&model)?;
    let ContainerStatus::Failed(ContainerFailure::Unreadable(reason)) =
        browser.container_status(id)
    else {
        return Err(format!("not failed: {:?}", browser.container_status(id)).into());
    };
    assert!(
        reason.contains("broken.socpak"),
        "names the Entry: {reason}"
    );
    assert!(
        !browser.is_expandable(TreeNode::container(id)),
        "no chevron for an unreadable container"
    );
    assert_eq!(browser.containers_used(), 0, "nothing held");
    Ok(())
}

#[test]
fn collapsing_a_container_while_it_opens_cancels_the_opening() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let id = container(&model, "packed.socpak")?;

    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    std::thread::sleep(std::time::Duration::from_millis(50));
    browser_mut(&mut model)?.poll();

    let browser = browser(&model)?;
    assert!(
        matches!(browser.container_status(id), ContainerStatus::Closed),
        "closed again: {:?}",
        browser.container_status(id)
    );
    assert_eq!(browser.containers_used(), 0, "the reservation is returned");
    Ok(())
}

#[test]
fn containers_not_in_use_are_closed_to_stay_within_the_budget() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let room = expand(&mut model, "room.socpak")?;
    let held = browser(&model)?.containers_used();
    browser_mut(&mut model)?.set_container_budget(held + 4096 + 64);

    let packed = expand(&mut model, "packed.socpak")?;
    let refused = matches!(
        browser(&model)?.container_status(packed),
        ContainerStatus::Failed(ContainerFailure::OverBudget)
    );
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(room));
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(packed));
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(packed));
    wait_until(&mut model, |model| socpak::settled(model, packed))?;

    let browser = browser(&model)?;
    assert!(refused, "the expanded room keeps its memory");
    assert!(
        matches!(browser.container_status(room), ContainerStatus::Closed),
        "the collapsed room is closed for the packed one"
    );
    assert!(
        matches!(browser.container_status(packed), ContainerStatus::Open),
        "open now: {:?}",
        browser.container_status(packed)
    );
    assert!(
        browser.containers_used() <= browser.container_budget(),
        "within the budget"
    );
    Ok(())
}

#[test]
fn checks_belong_to_the_shown_place() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    browser_mut(&mut model)?.set_checked(0, true);
    show_containers(&mut model)?;

    enter(&mut model, "room.socpak")?;
    let inside = browser(&model)?.checked_count();
    browser_mut(&mut model)?.set_checked(1, true);
    browser_mut(&mut model)?.set_checked(2, true);
    let archive_root = browser(&model)?.archive_index().root();
    browser_mut(&mut model)?.select_folder(archive_root);
    let outside = browser(&model)?.checked_count();
    enter(&mut model, "room.socpak")?;

    assert_eq!(inside, 0, "none of the container checked yet");
    assert_eq!(outside, 1, "the Archive keeps its checks");
    assert_eq!(
        browser(&model)?.checked_count(),
        2,
        "the container keeps its own"
    );
    Ok(())
}

#[test]
fn the_search_covers_the_shown_container() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;

    browser_mut(&mut model)?.search("*.cgf", || {});
    wait_until(&mut model, |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching())
    })?;

    assert_eq!(
        shown_paths(&model)?,
        ["room\\props\\chair.cgf", "room\\props\\table.cgf"],
        "matches inside the container"
    );
    Ok(())
}

#[test]
fn a_change_of_the_archive_drops_its_containers() -> TestResult {
    let (_dir, path, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let room = enter(&mut model, "room.socpak")?;

    archive()
        .entry(p4kx_testkit::TestEntry::stored("Data\\patch.txt", "patch"))
        .write_to(&path)?;
    let changed = model.check_archive();

    let browser = browser(&model)?;
    assert!(changed, "noticed");
    assert!(
        matches!(
            browser.container_status(room),
            ContainerStatus::ArchiveChanged
        ),
        "dropped"
    );
    assert_eq!(browser.place(), Place::Archive, "back to the Archive");
    assert_eq!(
        browser.node_name(browser.selected_node()),
        "ObjectContainers",
        "the folder that held it"
    );
    assert_eq!(browser.containers_used(), 0, "nothing held");
    Ok(())
}
