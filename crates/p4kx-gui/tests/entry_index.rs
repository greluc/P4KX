//! The path tree and the sort orders built from an opened Archive.

use std::error::Error;

use p4kx_extraction::Archive;
use p4kx_gui::{EntryIndex, Sort, SortColumn, SortDirection};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn index(entries: &[TestEntry]) -> Result<EntryIndex, Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    entries
        .iter()
        .cloned()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(EntryIndex::build(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?))
}

fn sample() -> Vec<TestEntry> {
    vec![
        TestEntry::zstd("Data\\Objects\\ship.cgf", vec![1; 300]),
        TestEntry::stored("Data\\Libs\\config.xml", vec![2; 20]),
        TestEntry::deflated("Data\\Objects\\Alpha.mtl", vec![3; 100]).encrypted(),
        TestEntry::stored("readme.txt", vec![4; 5]),
        TestEntry::stored("Data\\Objects\\Ships\\hull.cgf", vec![5; 50]),
    ]
}

fn names(index: &EntryIndex, ids: &[u32]) -> Vec<String> {
    ids.iter().map(|&id| index.name(id).to_owned()).collect()
}

const fn ascending(column: SortColumn) -> Sort {
    Sort {
        column,
        direction: SortDirection::Ascending,
    }
}

#[test]
fn the_root_holds_every_entry_and_the_top_level_folders() -> TestResult {
    let index = index(&sample())?;
    let root = index.folder(index.root());

    assert_eq!(index.len(), 5, "every Entry is indexed");
    assert_eq!(root.entry_count(), 5, "the root holds every Entry");
    let children: Vec<&str> = root
        .children()
        .iter()
        .map(|&child| index.folder(child).name())
        .collect();
    assert_eq!(children, ["Data"], "readme.txt is a file, not a folder");
    Ok(())
}

#[test]
fn folders_count_the_entries_below_them_and_list_subfolders_by_name() -> TestResult {
    let index = index(&sample())?;
    let objects = index
        .find_folder("Data\\Objects")
        .ok_or("no Data\\Objects")?;
    let data = index.find_folder("Data").ok_or("no Data")?;

    assert_eq!(index.folder(objects).entry_count(), 3, "Ships counts too");
    let children: Vec<&str> = index
        .folder(data)
        .children()
        .iter()
        .map(|&child| index.folder(child).name())
        .collect();
    assert_eq!(children, ["Libs", "Objects"], "subfolders sorted by name");
    assert_eq!(index.folder(objects).path(), "Data\\Objects", "full path");
    Ok(())
}

#[test]
fn entries_of_a_folder_sort_by_name_ignoring_case() -> TestResult {
    let index = index(&sample())?;
    let objects = index
        .find_folder("Data\\Objects")
        .ok_or("no Data\\Objects")?;

    let up = index.entries_in(objects, ascending(SortColumn::Name));
    let down = index.entries_in(
        objects,
        Sort {
            column: SortColumn::Name,
            direction: SortDirection::Descending,
        },
    );

    assert_eq!(
        names(&index, &up),
        ["Alpha.mtl", "hull.cgf", "ship.cgf"],
        "ascending"
    );
    assert_eq!(
        names(&index, &down),
        ["ship.cgf", "hull.cgf", "Alpha.mtl"],
        "descending"
    );
    Ok(())
}

#[test]
fn entries_sort_by_size_compression_and_encryption() -> TestResult {
    let index = index(&sample())?;
    let root = index.root();

    let by_size = index.entries_in(root, ascending(SortColumn::Size));
    let by_compression = index.entries_in(root, ascending(SortColumn::Compression));
    let by_encryption = index.entries_in(root, ascending(SortColumn::Encryption));

    assert_eq!(
        names(&index, &by_size),
        [
            "readme.txt",
            "config.xml",
            "hull.cgf",
            "Alpha.mtl",
            "ship.cgf"
        ],
        "by uncompressed size"
    );
    assert_eq!(
        names(&index, &by_compression).first().map(String::as_str),
        Some("config.xml"),
        "stored Entries come first, in path order"
    );
    assert_eq!(
        names(&index, &by_compression).last().map(String::as_str),
        Some("ship.cgf"),
        "zstd comes last"
    );
    assert_eq!(
        names(&index, &by_encryption).last().map(String::as_str),
        Some("Alpha.mtl"),
        "encrypted Entries come last"
    );
    Ok(())
}

#[test]
fn an_empty_archive_has_an_empty_root() -> TestResult {
    let index = index(&[])?;

    assert_eq!(index.len(), 0, "no Entries");
    assert!(
        index.folder(index.root()).children().is_empty(),
        "no folders"
    );
    assert!(
        index
            .entries_in(index.root(), ascending(SortColumn::Name))
            .is_empty(),
        "nothing shown"
    );
    Ok(())
}
