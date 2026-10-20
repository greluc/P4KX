//! Looking Entries up by path through a `PathIndex`.

use std::error::Error;

use p4kx_extraction::{Archive, PathIndex, PathKeys};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn entries_are_found_ignoring_case_and_separator_kind() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let mut builder = ArchiveBuilder::new();
    for index in 0..200 {
        builder = builder.entry(TestEntry::stored(
            format!("Data\\Objects\\Part{index}\\model.cgf"),
            vec![1; 4],
        ));
    }
    builder
        .entry(TestEntry::stored("Data\\Objects\\Cube.cgfm", vec![2; 4]))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let index = PathIndex::new(&archive);

    let found = index
        .find("data/objects/CUBE.CGFM")
        .ok_or("the companion")?;
    assert_eq!(found.path(), "Data\\Objects\\Cube.cgfm", "the stored path");
    assert_eq!(
        index
            .find("DATA\\objects/part117\\MODEL.cgf")
            .map(p4kx_extraction::Entry::path),
        Some("Data\\Objects\\Part117\\model.cgf"),
        "one of many"
    );
    assert!(
        index.find("Data\\Objects\\Cube.cgf").is_none(),
        "a prefix is no match"
    );
    assert!(index.find("").is_none(), "the empty path");
    Ok(())
}

#[test]
fn keys_built_once_serve_any_number_of_lent_indexes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Objects\\Cube.cgf", vec![1; 4]))
        .entry(TestEntry::stored("Data\\Objects\\Cube.cgfm", vec![2; 4]))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let keys = PathKeys::new(&archive);

    for _ in 0..3 {
        let index = keys.index(&archive);
        assert_eq!(
            index
                .find("data/objects/cube.CGFM")
                .map(p4kx_extraction::Entry::path),
            Some("Data\\Objects\\Cube.cgfm"),
            "found through the lent index"
        );
        assert!(index.find("Data\\Objects\\Sphere.cgf").is_none(), "absent");
    }
    Ok(())
}
