// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the container tests: a synthetic Archive with socpak containers, one nested in
//! another, one compressed and one corrupt.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use p4kx_gui::{Browser, ContainerId, ContainerStatus, Language, Model, Place, TreeNode};
use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const LAYOUT: &str = "<layout rooms=\"2\"/>\n";
pub(crate) const LAMP: &str = "<light colour=\"amber\"/>\n";

fn lights() -> TestContainer {
    TestContainer::new().entry(TestEntry::deflated("lamp.xml", LAMP))
}

fn room() -> TestContainer {
    TestContainer::new()
        .entry(TestEntry::deflated("room\\layout.xml", LAYOUT))
        .entry(TestEntry::stored("room\\props\\chair.cgf", "chair"))
        .entry(TestEntry::deflated("room\\props\\table.cgf", "table"))
        .container("room\\lights.socpak", &lights())
}

fn packed() -> TestContainer {
    TestContainer::new().entry(TestEntry::stored("crate.cgf", vec![3; 4096]))
}

/// Returns the Archive: a folder of three containers next to a plain Entry.
pub(crate) fn archive() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::stored(
            "Data\\ObjectContainers\\room.socpak",
            room().build(),
        ))
        .entry(TestEntry::zstd(
            "Data\\ObjectContainers\\packed.socpak",
            packed().build(),
        ))
        .entry(TestEntry::stored(
            "Data\\ObjectContainers\\broken.socpak",
            "not a ZIP archive",
        ))
        .entry(TestEntry::stored(
            "Data\\ObjectContainers\\notes.txt",
            "notes",
        ))
}

/// Writes `builder` and opens it in a window state; returns the folder holding it.
pub(crate) fn opened(
    builder: &ArchiveBuilder,
) -> Result<(TempDir, PathBuf, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.clone(), || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    Ok((dir, path, model))
}

/// Polls `model` until `done` holds.
pub(crate) fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(model) {
        if Instant::now() > deadline {
            return Err(format!("timed out: {model:?}").into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        model.poll_extraction();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

/// Returns the browser of `model`.
pub(crate) fn browser(model: &Model) -> Result<&Browser, Box<dyn Error>> {
    model.browser().ok_or_else(|| "no Archive open".into())
}

/// Returns the browser of `model` for changes.
pub(crate) fn browser_mut(model: &mut Model) -> Result<&mut Browser, Box<dyn Error>> {
    model.browser_mut().ok_or_else(|| "no Archive open".into())
}

/// Returns the node of the tree row called `name` below the expanded folders.
pub(crate) fn node(model: &Model, name: &str) -> Result<TreeNode, Box<dyn Error>> {
    let browser = browser(model)?;
    browser
        .tree_rows()
        .iter()
        .map(|row| row.node)
        .find(|&node| browser.node_name(node) == name)
        .ok_or_else(|| format!("no tree row {name}").into())
}

/// Returns the container of the tree row called `name`.
pub(crate) fn container(model: &Model, name: &str) -> Result<ContainerId, Box<dyn Error>> {
    node(model, name)?
        .as_container()
        .ok_or_else(|| format!("{name} is no container").into())
}

/// Returns whether the container `id` is no longer opening.
pub(crate) fn settled(model: &Model, id: ContainerId) -> bool {
    model
        .browser()
        .is_some_and(|browser| !matches!(browser.container_status(id), ContainerStatus::Opening))
}

/// Expands the folders `Data` and `ObjectContainers` of the Archive.
pub(crate) fn show_containers(model: &mut Model) -> TestResult {
    for name in ["Data", "ObjectContainers"] {
        let folder = node(model, name)?;
        browser_mut(model)?.toggle_expanded(folder);
    }
    Ok(())
}

/// Expands the container `name` and waits until it opened or failed.
pub(crate) fn expand(model: &mut Model, name: &str) -> Result<ContainerId, Box<dyn Error>> {
    let id = container(model, name)?;
    browser_mut(model)?.toggle_expanded(TreeNode::container(id));
    wait_until(model, |model| settled(model, id))?;
    Ok(id)
}

/// Selects the container `name`, waiting until its Entries are shown.
pub(crate) fn enter(model: &mut Model, name: &str) -> Result<ContainerId, Box<dyn Error>> {
    let id = container(model, name)?;
    browser_mut(model)?.select_folder(TreeNode::container(id));
    wait_until(model, |model| settled(model, id))?;
    if browser(model)?.place() != Place::Container(id) {
        return Err(format!("{name} is not shown").into());
    }
    Ok(id)
}

/// Returns the paths of the shown Entries in table order.
pub(crate) fn shown_paths(model: &Model) -> Result<Vec<String>, Box<dyn Error>> {
    let browser = browser(model)?;
    Ok(browser
        .shown()
        .iter()
        .filter_map(|&id| browser.index().entry(id))
        .map(|entry| entry.path().to_owned())
        .collect())
}

/// Returns the id of the shown Entry at `path`.
pub(crate) fn shown_id(model: &Model, path: &str) -> Result<u32, Box<dyn Error>> {
    let browser = browser(model)?;
    browser
        .shown()
        .iter()
        .copied()
        .find(|&id| {
            browser
                .index()
                .entry(id)
                .is_some_and(|entry| entry.path() == path)
        })
        .ok_or_else(|| format!("{path} is not shown").into())
}

/// Returns `path` below `dir` with backslashes as the platform separator.
pub(crate) fn below(dir: &Path, path: &str) -> PathBuf {
    path.split('\\')
        .fold(dir.to_path_buf(), |folder, part| folder.join(part))
}
