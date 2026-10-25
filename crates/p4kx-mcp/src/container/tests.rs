// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::path::Path;

use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};

use super::{Budget, Containers, ENTRY_COST, SEPARATOR};
use crate::archives::Archives;
use crate::container::Place;
use crate::error::ToolError;
use crate::gate::Gate;

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
        ..Containers::new(Gate::container())
    }
}

fn wait_for_waiters(containers: &Containers, count: usize) -> Result<(), ToolError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let waiting = containers.state.lock().is_ok_and(|state| {
            state.pending.iter().any(|(_, pending)| {
                pending.waiters.load(std::sync::atomic::Ordering::SeqCst) >= count
            })
        });
        if waiting {
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            return Err(ToolError::Refused("no caller waited".to_owned()));
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn weight_of(place: &Place<'_>) -> Result<u64, Box<dyn Error>> {
    match place {
        Place::Inner(inner) => Ok(inner.weight),
        Place::Main(_) => Err("not a container".into()),
    }
}

fn nested(dir: &Path, outer_stored: bool) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let room = TestContainer::new().entry(TestEntry::stored("room\\x.xml", "<x/>"));
    let outer = TestContainer::new()
        .entry(TestEntry::stored("outer\\a.xml", "<a/>"))
        .container("outer\\room.socpak", &room);
    let path = dir.join("Nested.p4k");
    let entry = if outer_stored {
        TestEntry::stored(OUTER, outer.build())
    } else {
        TestEntry::zstd(OUTER, outer.build())
    };
    ArchiveBuilder::new().entry(entry).write_to(&path)?;
    Ok(path)
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
    let decodes = std::sync::atomic::AtomicUsize::new(0);

    let all: Vec<_> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    shared.open_with(OUTER, || {
                        decodes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        wait_for_waiters(&shared, 3)?;
                        opened
                            .archive()
                            .open_container(entry)
                            .map_err(|_| ToolError::Stopped)
                    })
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
    assert_eq!(
        decodes.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "three callers waited for the decode of the fourth"
    );
    Ok(())
}

#[test]
fn a_waiter_gets_the_error_of_the_decode_it_waited_for() -> Result<(), Box<dyn Error>> {
    let shared = containers(Budget::default());
    let decodes = std::sync::atomic::AtomicUsize::new(0);

    let results: Vec<_> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    shared.open_with("broken", || {
                        decodes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        wait_for_waiters(&shared, 1)?;
                        Err(ToolError::Refused("cannot decode".to_owned()))
                    })
                })
            })
            .collect();
        threads
            .into_iter()
            .map(std::thread::ScopedJoinHandle::join)
            .collect()
    });

    for joined in results {
        let error = joined
            .map_err(|_| "thread panicked")?
            .err()
            .ok_or("no error")?;
        assert_eq!(error.message(), "cannot decode");
    }
    assert_eq!(decodes.load(std::sync::atomic::Ordering::SeqCst), 1);
    let state = shared.state.lock().map_err(|_| "poisoned")?;
    assert!(state.pending.is_empty() && state.kept.is_empty());
    drop(state);
    Ok(())
}

#[test]
fn a_stored_container_of_the_file_is_charged_for_its_entries_only() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archives = Archives::default();
    let opened = archives.get(&nested(dir.path(), true)?)?;

    let (place, _) = opened.place(&format!("{OUTER}::x"))?;

    assert_eq!(place.archive().resident_len(), 0, "read in place");
    assert_eq!(
        weight_of(&place)?,
        place.archive().entries().len() as u64 * ENTRY_COST
    );
    Ok(())
}

#[test]
fn a_stored_container_of_a_decoded_one_is_charged_the_buffer_it_keeps_alive()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archives = Archives::default();
    let opened = archives.get(&nested(dir.path(), false)?)?;

    let (outer, _) = opened.place(&format!("{OUTER}::x"))?;
    let room = format!("{OUTER}::outer\\room.socpak::x");
    let (child, _) = opened.place(&room)?;

    let buffer = outer.archive().resident_len();
    assert!(buffer > 0, "the compressed outer container is decoded");
    assert_eq!(child.archive().resident_len(), buffer, "shares the buffer");
    assert!(weight_of(&child)? >= buffer);
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
