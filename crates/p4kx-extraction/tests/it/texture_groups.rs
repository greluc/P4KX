// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The grouping of split texture Entries in the Entry listing.

use std::error::Error;

use p4kx_extraction::{Archive, Filter, TextureGroups};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const PATHS: [&str; 12] = [
    "Data\\readme.txt",
    "Data\\tex\\orphan.dds.1",
    "Data\\tex\\orphan.dds.a",
    "Data\\tex\\lonely.dds",
    "Data\\tex\\wall.dds",
    "Data\\tex\\wall.dds.1",
    "Data\\tex\\wall.dds.2",
    "Data\\tex\\wall.dds.a",
    "Data\\tex\\wall.dds.1a",
    "Data\\tex\\wall.dds.10",
    "data/tex/WALL.DDS.2a",
    "Data\\tex\\wall.png",
];

fn open(dir: &std::path::Path) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut builder = ArchiveBuilder::new();
    for (index, entry) in PATHS.iter().enumerate() {
        builder = builder.entry(TestEntry::stored(*entry, vec![1; index + 1]));
    }
    builder.write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

fn position(archive: &Archive, path: &str) -> Result<usize, Box<dyn Error>> {
    archive
        .entries()
        .iter()
        .position(|entry| entry.path() == path)
        .ok_or_else(|| format!("no {path}").into())
}

#[test]
fn a_header_and_its_parts_and_alpha_tail_form_one_group() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let groups = TextureGroups::build(&archive);

    assert_eq!(groups.len(), 1, "only wall.dds has parts");
    let group = groups.groups().first().ok_or("the group")?;
    let path = |position: usize| {
        archive
            .entries()
            .get(position)
            .map(p4kx_extraction::Entry::path)
    };
    assert_eq!(path(group.header()), Some("Data\\tex\\wall.dds"), "header");
    let parts: Vec<_> = group.parts().filter_map(path).collect();
    assert_eq!(
        parts,
        [
            "Data\\tex\\wall.dds.1",
            "Data\\tex\\wall.dds.2",
            "Data\\tex\\wall.dds.10",
            "Data\\tex\\wall.dds.a",
            "Data\\tex\\wall.dds.1a",
            "data/tex/WALL.DDS.2a",
        ],
        "colour parts by number, then the alpha tail"
    );
    assert_eq!(group.entry_count(), 7, "header and six parts");
    let sizes: u64 = group
        .members()
        .filter_map(|position| archive.entries().get(position))
        .map(p4kx_extraction::Entry::uncompressed_size)
        .sum();
    assert_eq!(group.uncompressed_size(), sizes, "total size");
    Ok(())
}

#[test]
fn orphans_headers_without_parts_and_other_entries_stay_single() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let groups = TextureGroups::build(&archive);

    for single in [
        "Data\\readme.txt",
        "Data\\tex\\orphan.dds.1",
        "Data\\tex\\orphan.dds.a",
        "Data\\tex\\lonely.dds",
        "Data\\tex\\wall.png",
    ] {
        let at = position(&archive, single)?;
        assert!(groups.group_of(at).is_none(), "{single} is in no group");
        assert!(!groups.is_part(at), "{single} is no part");
    }
    let header = position(&archive, "Data\\tex\\wall.dds")?;
    let part = position(&archive, "Data\\tex\\wall.dds.10")?;
    assert!(!groups.is_part(header), "the header is no part");
    assert!(groups.is_part(part), "a part");
    assert_eq!(
        groups.group_number(header),
        groups.group_number(part),
        "one group"
    );
    Ok(())
}

#[test]
fn a_filter_matching_one_part_selects_the_whole_group() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let groups = TextureGroups::build(&archive);

    let selected = Filter::new("*.dds.10").select_with_parts(&archive, &groups);
    assert_eq!(selected.len(), 7, "the group of wall.dds");
    let header = Filter::new("*wall.dds").select_with_parts(&archive, &groups);
    assert_eq!(header.len(), 7, "matching the header selects the parts");
    let none = Filter::new("*.txt").select_with_parts(&archive, &groups);
    assert_eq!(none.len(), 1, "an Entry of no group stays alone");
    assert!(
        Filter::new("missing")
            .select_with_parts(&archive, &groups)
            .is_empty(),
        "no match"
    );
    Ok(())
}

#[test]
fn an_archive_without_textures_has_no_groups() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", vec![1]))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let groups = TextureGroups::build(&archive);
    assert!(groups.is_empty(), "no groups");
    assert!(groups.group_of(0).is_none(), "no group");
    Ok(())
}
