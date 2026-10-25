// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The tree the parent indices of every `CryXml` Entry give against the one its child table gives,
//! over the Archive and the containers inside it.

use std::collections::BTreeMap;
use std::io::Read;
use std::time::Instant;

use p4kx_extraction::cryxml::{self, CryXmlError, Format, TreeComparison};
use p4kx_extraction::{Archive, Entry};
use p4kx_testkit::{real_archive_b_path, real_archive_path};
use rayon::prelude::*;

use super::{TestResult, report};

const MAX_CRYXML_SIZE: u64 = 64 << 20;
const PEEK_LEN: u64 = 8;
const CONTAINER_EXTENSION: &str = ".socpak";

#[derive(Default)]
struct Stats {
    scanned: u64,
    cryxml: u64,
    unreadable: u64,
    containers_opened: u64,
    containers_failed: u64,
    rejected: BTreeMap<&'static str, (u64, String)>,
    agree: u64,
    other_order: u64,
    equal_copies: u64,
    other_nodes: u64,
    outside_table: u64,
    attributes_not_sequential: u64,
    examples: BTreeMap<&'static str, String>,
}

impl Stats {
    fn merge(mut self, other: Self) -> Self {
        self.scanned += other.scanned;
        self.cryxml += other.cryxml;
        self.unreadable += other.unreadable;
        self.containers_opened += other.containers_opened;
        self.containers_failed += other.containers_failed;
        for (kind, (count, example)) in other.rejected {
            self.rejected.entry(kind).or_insert((0, example)).0 += count;
        }
        self.agree += other.agree;
        self.other_order += other.other_order;
        self.equal_copies += other.equal_copies;
        self.other_nodes += other.other_nodes;
        self.outside_table += other.outside_table;
        self.attributes_not_sequential += other.attributes_not_sequential;
        for (kind, example) in other.examples {
            self.examples.entry(kind).or_insert(example);
        }
        self
    }

    fn example(&mut self, kind: &'static str, path: &str) {
        self.examples.entry(kind).or_insert_with(|| path.to_owned());
    }

    fn compared(&mut self, path: &str, comparison: &TreeComparison) {
        if comparison.child_table_agrees() {
            self.agree += 1;
        }
        let kinds = [
            (comparison.children_in_other_order, "other order"),
            (comparison.children_naming_equal_copies, "equal copies"),
            (comparison.children_naming_other_nodes, "other nodes"),
            (comparison.child_range_outside_table, "range outside"),
            (comparison.attributes_not_sequential, "attributes"),
        ];
        for (count, kind) in kinds {
            if count > 0 {
                self.example(kind, path);
            }
        }
        self.other_order += u64::from(comparison.children_in_other_order > 0);
        self.equal_copies += u64::from(comparison.children_naming_equal_copies > 0);
        self.other_nodes += u64::from(comparison.children_naming_other_nodes > 0);
        self.outside_table += u64::from(comparison.child_range_outside_table > 0);
        self.attributes_not_sequential += u64::from(comparison.attributes_not_sequential > 0);
    }
}

const fn kind(error: &CryXmlError) -> &'static str {
    match error {
        CryXmlError::NotCryXml => "NotCryXml",
        CryXmlError::Header(_) => "Header",
        CryXmlError::Truncated { .. } => "Truncated",
        CryXmlError::TableOutOfBounds { .. } => "TableOutOfBounds",
        CryXmlError::BadString { .. } => "BadString",
        CryXmlError::BadAttributes { .. } => "BadAttributes",
        CryXmlError::BadParent { .. } => "BadParent",
        CryXmlError::NoRoot => "NoRoot",
    }
}

fn is_cryxml(archive: &Archive, entry: &Entry) -> bool {
    let mut head = Vec::with_capacity(8);
    archive
        .open_entry(entry)
        .is_ok_and(|reader| reader.take(PEEK_LEN).read_to_end(&mut head).is_ok())
        && matches!(cryxml::detect(&head), Some(Format::CryXml(_)))
}

fn check_entry(archive: &Archive, entry: &Entry, stats: &mut Stats) {
    stats.scanned += 1;
    let size = entry.uncompressed_size();
    if !(PEEK_LEN..=MAX_CRYXML_SIZE).contains(&size) || !is_cryxml(archive, entry) {
        return;
    }
    stats.cryxml += 1;
    let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
    let read = archive
        .open_entry(entry)
        .is_ok_and(|reader| reader.take(size).read_to_end(&mut bytes).is_ok());
    if !read {
        stats.unreadable += 1;
        return;
    }
    match cryxml::compare_trees(&bytes) {
        Ok(comparison) => stats.compared(entry.path(), &comparison),
        Err(error) => {
            stats
                .rejected
                .entry(kind(&error))
                .or_insert_with(|| (0, format!("{}: {error}", entry.path())))
                .0 += 1;
        }
    }
}

fn check_archive(archive: &Archive) -> Stats {
    archive
        .entries()
        .par_iter()
        .fold(Stats::default, |mut stats, entry| {
            if entry
                .path()
                .to_ascii_lowercase()
                .ends_with(CONTAINER_EXTENSION)
            {
                match archive.open_container(entry) {
                    Ok(container) => {
                        stats.containers_opened += 1;
                        stats = stats.merge(check_archive(&container));
                    }
                    Err(_) => stats.containers_failed += 1,
                }
            } else {
                check_entry(archive, entry, &mut stats);
            }
            stats
        })
        .reduce(Stats::default, Stats::merge)
}

fn print(name: &str, stats: &Stats, seconds: f64) {
    report(&format!(
        "{name}: {} Entries scanned (containers included), {} CryXml files, {} unreadable, \
         {} containers opened, {} failed, in {seconds:.1} s",
        stats.scanned,
        stats.cryxml,
        stats.unreadable,
        stats.containers_opened,
        stats.containers_failed,
    ));
    for (kind, (count, example)) in &stats.rejected {
        report(&format!("  rejected with {kind}: {count}, e.g. {example}"));
    }
    report(&format!(
        "  child table agrees: {}; other order: {}; equal copies: {}; other nodes: {}; \
         range outside the table: {}; attributes not sequential: {}",
        stats.agree,
        stats.other_order,
        stats.equal_copies,
        stats.other_nodes,
        stats.outside_table,
        stats.attributes_not_sequential,
    ));
    for (kind, path) in &stats.examples {
        report(&format!("  {kind}, e.g. {path}"));
    }
}

fn measure(name: &str, archive: &Archive) -> Stats {
    let started = Instant::now();
    let stats = check_archive(archive);
    print(name, &stats, started.elapsed().as_secs_f64());
    stats
}

fn assert_parent_indices_are_sound(stats: &Stats) {
    assert!(stats.cryxml > 1000, "only {} CryXml files", stats.cryxml);
    assert!(stats.rejected.is_empty(), "rejected: {:?}", stats.rejected);
    assert_eq!(stats.unreadable, 0, "unreadable files");
    assert_eq!(stats.other_order, 0, "child table in another order");
    assert_eq!(stats.outside_table, 0, "child range outside the table");
    assert_eq!(
        stats.attributes_not_sequential, 0,
        "attributes not sequential"
    );
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_parent_indices_give_a_sound_tree_where_the_child_table_may_not() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    assert_parent_indices_are_sound(&measure("Archive", &archive));
    if let Some(path) = real_archive_b_path() {
        let second = Archive::open_with_key(path, Some(p4kx_testkit::real_archive_key()?))?;
        assert_parent_indices_are_sound(&measure("second Archive", &second));
    }
    Ok(())
}
