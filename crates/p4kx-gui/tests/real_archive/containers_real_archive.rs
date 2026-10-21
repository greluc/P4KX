//! The socpak containers of a real Archive in the path tree; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Browser, ContainerFailure, ContainerId, ContainerStatus, Language, Model, Place, Preview,
    TreeNode,
};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

fn wait_until(model: &mut Model, limit: Duration, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + limit;
    while !done(model) {
        if Instant::now() > deadline {
            return Err("timed out".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

fn browser(model: &mut Model) -> Result<&mut Browser, Box<dyn Error>> {
    model.browser_mut().ok_or_else(|| "no Archive open".into())
}

/// Opens the real Archive and expands the folders down to the one holding the most socpaks;
/// returns the containers of that folder.
fn opened() -> Result<(Model, Vec<ContainerId>), Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    model.open(real_archive_path()?, || {});
    wait_until(&mut model, Duration::from_secs(120), |model| {
        model.browser().is_some() || model.failure().is_some()
    })?;
    let browser = browser(&mut model)?;
    let index = browser.archive_index();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for path in (0..u32::try_from(index.len())?)
        .filter_map(|id| index.entry(id))
        .map(p4kx_extraction::Entry::path)
        .filter(|path| path.to_ascii_lowercase().ends_with(".socpak"))
    {
        *counts
            .entry(path.rsplit_once('\\').map_or("", |(folder, _)| folder))
            .or_default() += 1;
    }
    let holder = counts
        .into_iter()
        .max_by_key(|&(folder, count)| (count, std::cmp::Reverse(folder)))
        .map(|(folder, _)| folder.to_owned())
        .ok_or("the Archive holds no socpak")?;
    let mut prefix = String::new();
    for part in holder.split('\\') {
        if !prefix.is_empty() {
            prefix.push('\\');
        }
        prefix.push_str(part);
        let folder = browser
            .archive_index()
            .find_folder(&prefix)
            .ok_or("no folder")?;
        if !browser.is_expanded(folder) {
            browser.toggle_expanded(folder);
        }
    }
    let containers = browser
        .tree_rows()
        .iter()
        .filter_map(|row| row.node.as_container())
        .collect();
    Ok((model, containers))
}

fn settled(model: &Model, id: ContainerId) -> bool {
    model
        .browser()
        .is_some_and(|browser| !matches!(browser.container_status(id), ContainerStatus::Opening))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_socpak_of_a_real_archive_opens_and_previews_its_entries() -> TestResult {
    let (mut model, containers) = opened()?;
    let id = *containers.first().ok_or("no container row")?;

    browser(&mut model)?.select_folder(TreeNode::container(id));
    wait_until(&mut model, Duration::from_secs(10), |model| {
        settled(model, id)
    })?;
    let browser_now = browser(&mut model)?;
    let status = format!("{:?}", browser_now.container_status(id));
    assert_eq!(browser_now.place(), Place::Container(id), "shown");
    assert!(
        !browser_now.shown().is_empty(),
        "lists its Entries: {status}"
    );
    let first = *browser_now.shown().first().ok_or("nothing shown")?;
    browser_now.select_entry(first, || {});
    wait_until(&mut model, Duration::from_secs(10), |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })?;

    let read = model
        .browser()
        .and_then(Browser::preview)
        .and_then(Preview::contents)
        .is_some();
    assert!(read, "the preview reads an Entry inside the container");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_socpak_of_a_real_folder_opens_within_the_budget() -> TestResult {
    let (mut model, containers) = opened()?;

    for &id in &containers {
        browser(&mut model)?.toggle_expanded(TreeNode::container(id));
    }
    wait_until(&mut model, Duration::from_secs(120), |model| {
        containers.iter().all(|&id| settled(model, id))
    })?;

    let browser = browser(&mut model)?;
    let unreadable: Vec<String> = containers
        .iter()
        .filter_map(|&id| match browser.container_status(id) {
            ContainerStatus::Failed(ContainerFailure::Unreadable(reason)) => Some(reason.clone()),
            _ => None,
        })
        .collect();
    let open = containers
        .iter()
        .filter(|&&id| matches!(browser.container_status(id), ContainerStatus::Open))
        .count();
    assert!(unreadable.is_empty(), "every socpak reads: {unreadable:?}");
    assert!(open > 0, "{open} of {} open", containers.len());
    assert!(
        browser.containers_used() <= browser.container_budget(),
        "within the budget"
    );
    Ok(())
}
