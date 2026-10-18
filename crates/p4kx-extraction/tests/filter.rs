//! The Filter that selects Entries for search, Extraction and the GUI.

use std::error::Error;

use p4kx_extraction::{Archive, Entry, Filter};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const PATH: &str = "Data\\Libs\\Config\\defaultProfile.xml";

fn matches(filter: &str, path: &str) -> bool {
    Filter::new(filter).matches(path)
}

#[test]
fn text_without_wildcards_is_a_substring_anywhere_in_the_path() {
    for filter in ["defaultProfile", "Libs\\Config", "Data", ".xml", PATH] {
        assert!(matches(filter, PATH), "{filter:?} is part of the path");
    }
    assert!(
        !matches("Profiles", PATH),
        "Profiles is not part of the path"
    );
    assert!(!Filter::new("Libs").is_glob(), "no wildcard, no glob");
}

#[test]
fn substrings_ignore_ascii_case() {
    assert!(matches("DEFAULTPROFILE.XML", PATH), "upper case filter");
    assert!(
        matches("libs", "DATA\\LIBS\\A.XML"),
        "lower case filter, upper case path"
    );
}

#[test]
fn either_separator_matches_both_separators() {
    assert!(matches("Libs/Config", PATH), "slash matches backslash");
    assert!(
        matches("Libs\\Config", "Data/Libs/Config/a.xml"),
        "backslash matches slash"
    );
}

#[test]
fn an_empty_filter_matches_every_path() {
    assert!(matches("", PATH), "empty filter");
    assert!(matches("", ""), "empty filter, empty path");
}

#[test]
fn whitespace_is_part_of_the_filter() {
    assert!(!matches(" Libs", PATH), "leading space is not trimmed");
    assert!(matches("my file", "Data\\my file.txt"), "inner space");
}

#[test]
fn a_star_or_question_mark_makes_a_glob_over_the_full_path() {
    assert!(Filter::new("*.xml").is_glob(), "star makes a glob");
    assert!(Filter::new("a?c").is_glob(), "question mark makes a glob");
    assert!(matches("*.xml", PATH), "star at the start");
    assert!(
        !matches("Libs*", PATH),
        "a glob is anchored at the start of the path"
    );
    assert!(
        !matches("*Libs", PATH),
        "a glob is anchored at the end of the path"
    );
    assert!(matches("*Libs*", PATH), "stars on both sides");
}

#[test]
fn a_star_matches_any_run_of_characters_including_separators() {
    assert!(matches("Data\\*.xml", PATH), "star crosses separators");
    assert!(matches("Data*Profile*", PATH), "several stars");
    assert!(matches("Data**.xml", PATH), "double star is a star");
    assert!(matches("*", PATH), "a lone star matches everything");
    assert!(matches("*", ""), "a lone star matches the empty path");
    assert!(matches("Data\\*", "Data\\"), "a star matches the empty run");
}

#[test]
fn a_question_mark_matches_exactly_one_character() {
    assert!(matches("Data\\a?c.xml", "Data\\abc.xml"), "one character");
    assert!(
        matches("Data?a.xml", "Data\\a.xml"),
        "a separator is one character"
    );
    assert!(
        matches("Data\\?.xml", "Data\\ä.xml"),
        "a non-ASCII letter is one character"
    );
    assert!(!matches("Data\\a?c.xml", "Data\\ac.xml"), "not zero");
    assert!(!matches("Data\\a?c.xml", "Data\\abbc.xml"), "not two");
}

#[test]
fn globs_ignore_ascii_case_and_separator_kind() {
    assert!(matches("data/libs/*.XML", PATH), "case and separator");
    assert!(
        matches("Data\\*\\a.xml", "Data/x/a.xml"),
        "backslash in the glob matches a slash in the path"
    );
}

#[test]
fn other_glob_characters_are_literal() {
    assert!(
        matches("*[1].xml", "Data\\a[1].xml"),
        "brackets are literal"
    );
    assert!(!matches("*[1].xml", "Data\\a1.xml"), "no character class");
    assert!(matches("{a}", "Data\\{a}.xml"), "braces are literal");
}

#[test]
fn backtracking_finds_a_later_match() {
    assert!(
        matches("*a*b", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaab"),
        "the last b ends the path"
    );
    assert!(
        !matches("*a*a*a*a*a*b", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        "no b at all"
    );
}

#[test]
fn the_filter_keeps_its_text() {
    assert_eq!(
        Filter::new("Data/*.xml").as_str(),
        "Data/*.xml",
        "the text as given"
    );
    assert_eq!(
        Filter::new("Data/*.xml").to_string(),
        "Data/*.xml",
        "displayed as given"
    );
}

#[test]
fn select_returns_the_matching_entries_in_archive_order() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\b.xml", b"b".to_vec()))
        .entry(TestEntry::stored("Data\\a.txt", b"a".to_vec()))
        .entry(TestEntry::stored("Data\\a.xml", b"a".to_vec()))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;

    let selected: Vec<&str> = Filter::new("*.xml")
        .select(&archive)
        .into_iter()
        .map(Entry::path)
        .collect();

    assert_eq!(
        selected,
        vec!["Data\\b.xml", "Data\\a.xml"],
        "matches in Archive order"
    );
    assert!(
        Filter::new("missing").select(&archive).is_empty(),
        "no match, no Entries"
    );
    Ok(())
}
