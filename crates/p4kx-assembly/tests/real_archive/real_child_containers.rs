// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Real socpaks with `ChildObjectContainers`: the descriptions read and the children are placed.

use std::error::Error;
use std::io::Read;
use std::sync::Mutex;

use p4kx_assembly::{Interior, InteriorOptions, InteriorPartKind};
use p4kx_extraction::{Archive, select_entries};
use p4kx_model::ObjectContainer;
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const ARENA: &str = "Data\\ObjectContainers\\EA\\ClioIslands\\clioislands_arena.socpak";

fn description(root: &Archive, stem: &str) -> Result<ObjectContainer, Box<dyn Error>> {
    let file = root
        .entries()
        .iter()
        .find(|file| file.path().eq_ignore_ascii_case(&format!("{stem}.xml")))
        .ok_or("no description")?;
    let mut bytes = Vec::new();
    root.open_entry(file)?.read_to_end(&mut bytes)?;
    Ok(ObjectContainer::parse(&bytes)?)
}

#[derive(Default, Debug)]
struct Census {
    containers: usize,
    children: usize,
    external: usize,
    malformed: usize,
    without_transform: usize,
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts go into the PR body")]
fn the_children_of_real_containers_have_numbers_for_their_placement() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let census = Mutex::new(Census::default());
    let socpaks: Vec<_> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".socpak"))
        .collect();
    socpaks.par_iter().for_each(|entry| {
        let Ok(root) = archive.open_container(entry) else {
            return;
        };
        let name = entry.path().rsplit('\\').next().unwrap_or_default();
        let stem = name.strip_suffix(".socpak").unwrap_or(name);
        let Ok(container) = description(&root, stem) else {
            return;
        };
        if container.children.is_empty() {
            return;
        }
        let Ok(mut census) = census.lock() else {
            return;
        };
        census.containers += 1;
        census.children += container.children.len();
        for child in &container.children {
            census.external += usize::from(child.external);
            census.malformed += usize::from(child.malformed);
            census.without_transform += usize::from(child.transform().is_none());
        }
    });
    let census = census.into_inner()?;
    println!("{census:#?}");
    assert!(census.containers > 1000, "{census:?}");
    assert!(census.external > 0, "{census:?}");
    assert_eq!((census.malformed, census.without_transform), (0, 0));
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_container_with_a_child_places_it_at_its_declared_position() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let found = select_entries(&archive, &[ARENA])?;
    let entry = found.first().ok_or("no arena container")?;
    let root = archive.open_container(entry)?;
    let declared = description(&root, "clioislands_arena")?;
    declared.children.first().ok_or("no children")?;

    let interior = Interior::build(
        &[&archive, &root],
        entry.path(),
        &InteriorOptions::default(),
    )?;

    assert!(interior.issues().is_empty(), "{:?}", interior.issues());
    for child in &declared.children {
        let file = child.name.rsplit(['/', '\\']).next().unwrap_or_default();
        let stem = file.strip_suffix(".socpak").unwrap_or(file);
        let part = interior
            .parts()
            .iter()
            .find(|part| part.kind() == InteriorPartKind::Container && part.name() == stem)
            .ok_or_else(|| format!("no part for the child {}", child.name))?;
        let expected = child.transform().ok_or("no transform")?.translation();
        assert_eq!(part.world().translation(), expected, "{}", child.name);
    }
    Ok(())
}
