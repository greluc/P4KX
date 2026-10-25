// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::path::Path;

use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};

use super::{Budget, Containers, ModelGate, SEPARATOR};
use crate::archives::Archives;

const OUTER: &str = "Data\\outer.socpak";
const BROKEN: &str = "Data\\broken.socpak";

fn install(dir: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let outer = TestContainer::new()
        .entry(TestEntry::stored("outer\\a.xml", "<a/>"))
        .entry(TestEntry::stored("outer\\b.xml", "<b/>"));
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(OUTER, outer.build()))
        .entry(TestEntry::stored(BROKEN, "no container"))
        .write_to(&path)?;
    Ok(path)
}

fn containers(budget: Budget) -> Containers {
    Containers {
        budget,
        ..Containers::new(ModelGate::default())
    }
}

#[test]
fn the_separator_splits_a_chain_into_containers_and_the_rest() {
    let parts: Vec<&str> = "a.socpak::b.socpak::c\\d.xml".split(SEPARATOR).collect();
    assert_eq!(parts, ["a.socpak", "b.socpak", "c\\d.xml"]);
    assert_eq!(
        "a.socpak::".split(SEPARATOR).collect::<Vec<_>>(),
        ["a.socpak", ""],
        "an empty rest is the container's top level"
    );
}

#[test]
fn a_kept_container_is_returned_again() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archives = Archives::default();
    let opened = archives.get(&install(dir.path())?)?;

    let (first, _) = opened.place(&format!("{OUTER}::outer\\a.xml"))?;
    let second_path = format!("{OUTER}::outer\\b.xml");
    let (second, rest) = opened.place(&second_path)?;

    assert_eq!(rest, "outer\\b.xml");
    assert!(
        std::ptr::eq(first.archive(), second.archive()),
        "decoded once"
    );
    assert!(
        second.index().find("outer/b.xml").is_some(),
        "found through the index, whatever the separator"
    );
    Ok(())
}

#[test]
fn a_container_over_the_size_limit_is_not_kept() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archives = Archives::default();
    let opened = archives.get(&install(dir.path())?)?;
    let entry = opened
        .index()
        .find(OUTER)
        .ok_or("outer container missing")?;
    let none = containers(Budget {
        largest: 0,
        ..Budget::default()
    });

    let first = none.open(opened.archive(), entry, OUTER)?;
    let second = none.open(opened.archive(), entry, OUTER)?;

    assert!(!std::sync::Arc::ptr_eq(&first, &second), "decoded again");
    assert!(none.state.lock().is_ok_and(|state| state.kept.is_empty()));
    Ok(())
}

#[test]
fn the_byte_budget_drops_the_least_recently_used_container() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archives = Archives::default();
    let opened = archives.get(&install(dir.path())?)?;
    let entry = opened
        .index()
        .find(OUTER)
        .ok_or("outer container missing")?;
    let one = containers(Budget::default());
    let weight = one.open(opened.archive(), entry, "one")?.weight;
    let tight = containers(Budget {
        bytes: weight + weight / 2,
        ..Budget::default()
    });

    let old = tight.open(opened.archive(), entry, "old")?;
    let new = tight.open(opened.archive(), entry, "new")?;

    let state = tight.state.lock().map_err(|_| "poisoned")?;
    assert_eq!(state.kept.len(), 1, "only one fits the budget");
    assert!(std::sync::Arc::ptr_eq(&state.kept[0], &new));
    drop(state);
    drop(old);
    Ok(())
}

#[test]
fn calls_at_the_same_time_share_one_container() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archives = Archives::default();
    let opened = archives.get(&install(dir.path())?)?;
    let entry = opened
        .index()
        .find(OUTER)
        .ok_or("outer container missing")?;
    let shared = containers(Budget::default());
    let barrier = std::sync::Barrier::new(4);

    let all: Vec<_> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    shared.open(opened.archive(), entry, OUTER)
                })
            })
            .collect();
        threads
            .into_iter()
            .map(std::thread::ScopedJoinHandle::join)
            .collect()
    });

    let mut inners = Vec::new();
    for joined in all {
        inners.push(joined.map_err(|_| "thread panicked")??);
    }
    assert!(
        inners
            .windows(2)
            .all(|pair| std::sync::Arc::ptr_eq(&pair[0], &pair[1]))
    );
    Ok(())
}

#[test]
fn a_failed_decode_is_forgotten_and_reported_to_its_caller() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archives = Archives::default();
    let opened = archives.get(&install(dir.path())?)?;

    assert!(opened.place(&format!("{BROKEN}::x")).is_err());
    assert!(
        opened.place(&format!("{BROKEN}::x")).is_err(),
        "tried again"
    );
    let state = opened.containers().state.lock().map_err(|_| "poisoned")?;
    assert!(state.pending.is_empty() && state.kept.is_empty());
    drop(state);
    Ok(())
}
