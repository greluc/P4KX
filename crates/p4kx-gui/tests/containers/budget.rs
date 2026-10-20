//! The memory budget of the open containers: reservations, closing, refusing and retrying.

use p4kx_gui::{ContainerFailure, ContainerStatus, TreeNode};

use crate::socpak::{
    self, TestResult, archive, browser, browser_mut, container, enter, expand, node, opened,
    show_containers, wait_until,
};

#[test]
fn collapsing_a_container_while_it_opens_keeps_its_reservation_until_the_thread_ends() -> TestResult
{
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let id = container(&model, "packed.socpak")?;

    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    let reserved = browser(&model)?.containers_used();
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));

    let browser = browser(&model)?;
    assert!(
        matches!(browser.container_status(id), ContainerStatus::Closed),
        "closed again: {:?}",
        browser.container_status(id)
    );
    assert!(reserved > 0, "reserved while opening");
    assert_eq!(
        browser.containers_used(),
        reserved,
        "the thread still decodes, so the reservation stays"
    );
    wait_until(&mut model, |model| {
        model
            .browser()
            .is_some_and(|browser| browser.containers_used() == 0)
    })?;
    Ok(())
}

#[test]
fn expanding_a_container_that_is_still_draining_takes_its_opening_back() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let id = container(&model, "packed.socpak")?;
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    let reserved = browser(&model)?.containers_used();

    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(id));
    wait_until(&mut model, |model| socpak::settled(model, id))?;

    let browser = browser(&model)?;
    assert!(
        matches!(browser.container_status(id), ContainerStatus::Open),
        "open: {:?}",
        browser.container_status(id)
    );
    assert!(
        browser.containers_used() < reserved * 2,
        "charged once, not twice"
    );
    Ok(())
}

#[test]
fn a_container_the_budget_closed_does_not_come_back_expanded() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let room = expand(&mut model, "room.socpak")?;
    let folder = node(&model, "room")?;
    browser_mut(&mut model)?.toggle_expanded(folder);
    let lights = expand(&mut model, "lights.socpak")?;
    let held = browser(&model)?.containers_used();
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(room));
    browser_mut(&mut model)?.set_container_budget(held + 4095);

    let packed = expand(&mut model, "packed.socpak")?;
    let before = browser(&model)?;
    assert!(
        matches!(before.container_status(packed), ContainerStatus::Open),
        "open: {:?}",
        before.container_status(packed)
    );
    assert!(
        matches!(before.container_status(room), ContainerStatus::Closed)
            && matches!(before.container_status(lights), ContainerStatus::Closed),
        "the budget closed both"
    );

    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(packed));
    expand(&mut model, "room.socpak")?;

    let browser = browser(&model)?;
    assert!(
        matches!(browser.container_status(room), ContainerStatus::Open),
        "open again"
    );
    assert!(!browser.is_expanded(folder), "its folders are collapsed");
    assert!(
        !browser.is_expanded(TreeNode::container(lights)),
        "so is the container inside"
    );
    assert!(
        node(&model, "lights.socpak").is_err(),
        "nothing of the old state is shown"
    );
    assert!(node(&model, "room").is_ok(), "the folder itself is there");
    Ok(())
}

#[test]
fn a_refused_container_keeps_its_chevron_and_retries() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    browser_mut(&mut model)?.set_container_budget(100);

    let id = expand(&mut model, "packed.socpak")?;

    let browser = browser(&model)?;
    assert!(
        matches!(
            browser.container_status(id),
            ContainerStatus::Failed(ContainerFailure::OverBudget)
        ),
        "refused"
    );
    assert!(browser.is_expandable(TreeNode::container(id)), "chevron");
    assert!(
        !browser.is_expanded(TreeNode::container(id)),
        "offers Expand"
    );
    Ok(())
}

#[test]
fn the_checks_of_a_closed_container_are_kept() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    let room = enter(&mut model, "room.socpak")?;
    browser_mut(&mut model)?.set_checked(1, true);
    let archive_root = browser(&model)?.archive_index().root();
    browser_mut(&mut model)?.select_folder(archive_root);
    let held = browser(&model)?.containers_used();
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(room));
    browser_mut(&mut model)?.set_container_budget(held + 4095);

    let packed = expand(&mut model, "packed.socpak")?;
    let closed = matches!(
        browser(&model)?.container_status(room),
        ContainerStatus::Closed
    );
    browser_mut(&mut model)?.toggle_expanded(TreeNode::container(packed));
    enter(&mut model, "room.socpak")?;

    assert!(closed, "the budget closed the room");
    assert_eq!(browser(&model)?.checked_count(), 1, "its check survived");
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
