// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The exclusion Filters that leave Entries out of a selection.

use std::error::Error;

use p4kx_extraction::{
    Archive, Entry, Exclusion, Filter, FilterError, LeftOut, MAX_EXCLUSIONS, MAX_FILTER_LEN,
    MAX_FILTER_WILDCARDS, TextureGroups,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const PATHS: [&str; 10] = [
    "Data\\Objects\\ship.cga",
    "Data\\Objects\\ship_lod1.cgam",
    "Data\\Objects\\ship_LOD2.cgam",
    "Data\\Objects\\Sub\\part.cga",
    "Data\\readme.txt",
    "Data\\tex\\wall.dds",
    "Data\\tex\\wall.dds.1",
    "Data\\tex\\wall.dds.a",
    "Data\\tex\\floor.dds",
    "Data\\tex\\floor.dds.1",
];

fn open(dir: &std::path::Path) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut builder = ArchiveBuilder::new();
    for path in PATHS {
        builder = builder.entry(TestEntry::stored(path, "x"));
    }
    builder.write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

fn exclusion(texts: &[&str]) -> Result<Exclusion, FilterError> {
    Exclusion::parse(texts)
}

fn selected(
    archive: &Archive,
    include: &str,
    exclude: &[&str],
) -> Result<Vec<String>, Box<dyn Error>> {
    let groups = TextureGroups::build(archive);
    let entries = Filter::new(include).select_with_parts(archive, &groups);
    let left = leave_out(&exclusion(exclude)?, entries);
    Ok(paths(&left.kept))
}

fn leave_out<'a>(exclusion: &Exclusion, entries: Vec<&'a Entry>) -> LeftOut<&'a Entry> {
    exclusion.leave_out(entries, |entry: &&'a Entry| -> &'a str { entry.path() })
}

fn paths(entries: &[&Entry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry.path().to_owned())
        .collect()
}

#[test]
fn an_entry_is_selected_when_the_include_matches_and_no_exclude_does() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;

    let kept = selected(&archive, "Data\\Objects\\*", &["*_lod?.cgam"])?;

    assert_eq!(
        kept,
        ["Data\\Objects\\ship.cga", "Data\\Objects\\Sub\\part.cga"],
        "the level-of-detail companions are left out, the rest stays"
    );
    Ok(())
}

#[test]
fn an_exclude_ignores_case_and_separator_kind() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;

    let kept = selected(&archive, "objects", &["OBJECTS/sub"])?;

    assert!(
        !kept.iter().any(|path| path.contains("Sub")),
        "a slash and other case still match: {kept:?}"
    );
    assert_eq!(kept.len(), 3, "ship and its two companions remain");
    Ok(())
}

#[test]
fn every_exclude_filter_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;

    let kept = selected(&archive, "Objects", &["_lod1", "Sub\\"])?;

    assert_eq!(
        kept,
        ["Data\\Objects\\ship.cga", "Data\\Objects\\ship_LOD2.cgam"],
        "each exclude removes its own Entries"
    );
    Ok(())
}

#[test]
fn an_exclude_that_removes_everything_leaves_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let groups = TextureGroups::build(&archive);
    let entries = Filter::new("Objects").select_with_parts(&archive, &groups);
    let found = entries.len();

    let left = leave_out(&exclusion(&["data"])?, entries);

    assert!(left.kept.is_empty(), "everything lies below Data");
    assert_eq!(left.removed, found, "all of them were left out");
    Ok(())
}

#[test]
fn without_a_filter_nothing_is_left_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let all: Vec<&Entry> = archive.entries().iter().collect();

    let left = leave_out(&Exclusion::default(), all.clone());

    assert_eq!(left.kept, all, "the same Entries in the same order");
    assert_eq!(left.removed, 0, "none removed");
    Ok(())
}

#[test]
fn an_excluded_texture_header_takes_its_parts_with_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;

    let kept = selected(&archive, "Data\\tex\\", &["wall.dds"])?;

    assert_eq!(
        kept,
        ["Data\\tex\\floor.dds", "Data\\tex\\floor.dds.1"],
        "wall.dds and its part and alpha tail are gone, floor stays whole"
    );
    Ok(())
}

#[test]
fn an_excluded_part_of_a_kept_texture_is_reported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let groups = TextureGroups::build(&archive);
    let entries = Filter::new("wall").select_with_parts(&archive, &groups);

    let left = leave_out(&exclusion(&["*.dds.?"])?, entries);

    assert_eq!(
        paths(&left.kept),
        ["Data\\tex\\wall.dds"],
        "the header stays"
    );
    assert_eq!(
        paths(&left.parts_of_kept_textures),
        ["Data\\tex\\wall.dds.1", "Data\\tex\\wall.dds.a"],
        "both parts are named rather than dropped silently"
    );
    Ok(())
}

#[test]
fn a_dropped_header_reports_no_parts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let groups = TextureGroups::build(&archive);
    let entries = Filter::new("wall").select_with_parts(&archive, &groups);

    let left = leave_out(&exclusion(&["wall"])?, entries);

    assert!(left.kept.is_empty(), "the whole texture is out");
    assert!(
        left.parts_of_kept_textures.is_empty(),
        "no texture is kept, so no part is reported"
    );
    Ok(())
}

#[test]
fn handles_to_entries_are_left_out_by_the_path_they_stand_for() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(dir.path())?;
    let ids: Vec<usize> = (0..archive.entries().len()).collect();
    let path_of = |id: &usize| archive.entries().get(*id).map_or("", Entry::path);

    let left = exclusion(&["_lod"])?.leave_out(ids, path_of);

    let kept: Vec<&str> = left.kept.iter().map(path_of).collect();
    assert_eq!(left.removed, 2, "the two companions");
    assert!(
        kept.iter().all(|path| !path.contains("_lod")) && kept.len() == PATHS.len() - 2,
        "the rest keeps its order: {kept:?}"
    );
    Ok(())
}

#[test]
fn an_empty_exclude_is_refused() {
    assert_eq!(
        exclusion(&["Sub", ""]).err(),
        Some(FilterError::EmptyExclusion),
        "an empty text matches every path"
    );
}

#[test]
fn filter_text_is_bounded_in_length_and_wildcards() {
    let long = "a".repeat(MAX_FILTER_LEN + 1);
    assert_eq!(
        Filter::parse(&long).err(),
        Some(FilterError::TooLong {
            len: MAX_FILTER_LEN + 1
        }),
        "too long"
    );
    assert!(
        Filter::parse(&"a".repeat(MAX_FILTER_LEN)).is_ok(),
        "at the limit"
    );

    let wild = "*a".repeat(MAX_FILTER_WILDCARDS + 1);
    assert_eq!(
        Filter::parse(&wild).err(),
        Some(FilterError::TooManyWildcards {
            count: MAX_FILTER_WILDCARDS + 1
        }),
        "too many wildcards"
    );
    assert!(
        Filter::parse(&"?".repeat(MAX_FILTER_WILDCARDS)).is_ok(),
        "at the limit"
    );
    assert_eq!(
        exclusion(&[&long]).err(),
        Some(FilterError::TooLong {
            len: MAX_FILTER_LEN + 1
        }),
        "an exclude is bounded too"
    );
}

#[test]
fn the_number_of_excludes_is_bounded() {
    let texts = vec!["a"; MAX_EXCLUSIONS + 1];
    assert_eq!(
        Exclusion::parse(&texts).err(),
        Some(FilterError::TooManyExclusions {
            count: MAX_EXCLUSIONS + 1
        }),
        "one more than allowed"
    );
    assert!(
        Exclusion::parse(&texts[..MAX_EXCLUSIONS]).is_ok(),
        "exactly the limit"
    );
}

#[test]
fn a_glob_with_many_stars_ends_quickly_on_a_long_path() {
    let pattern = "*a".repeat(MAX_FILTER_WILDCARDS / 2);
    let path = "a".repeat(4096) + "b";
    let filter = Filter::parse(&pattern).expect("within limits");
    assert!(!filter.matches(&path), "no path ends in a");
}
