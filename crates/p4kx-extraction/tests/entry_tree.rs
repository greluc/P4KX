//! The folder tree of an Archive that the GUI and the MCP server list folders from.

use std::error::Error;

use p4kx_extraction::{Archive, Entry, EntryTree, FolderId};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn open(paths: &[&str]) -> Result<(tempfile::TempDir, Archive), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    paths
        .iter()
        .map(|path| TestEntry::stored(*path, b"x".to_vec()))
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    Ok((folder, archive))
}

fn child_names(tree: &EntryTree, folder: FolderId) -> Vec<&str> {
    tree.folder(folder)
        .children()
        .iter()
        .map(|&child| tree.folder(child).name())
        .collect()
}

fn file_paths<'a>(tree: &EntryTree, archive: &'a Archive, folder: FolderId) -> Vec<&'a str> {
    tree.files(folder)
        .iter()
        .filter_map(|&id| tree.entry(archive, id))
        .map(Entry::path)
        .collect()
}

#[test]
fn folders_that_differ_only_by_case_are_one_folder_named_by_its_first_entry() -> TestResult {
    let (_folder, archive) = open(&[
        "Data\\objects\\b.cgf",
        "Data\\Objects\\a.cgf",
        "Data\\Libs\\c.xml",
        "data\\OBJECTS\\Ships\\d.cgf",
    ])?;
    let tree = EntryTree::build(&archive);
    assert_eq!(child_names(&tree, tree.root()), ["Data"], "one top folder");
    let data = tree.find_folder("DATA").ok_or("no Data")?;
    assert_eq!(child_names(&tree, data), ["Libs", "Objects"], "case folded");
    let objects = tree.find_folder("data/objects").ok_or("no Objects")?;
    assert_eq!(
        tree.folder(objects).path(),
        "Data\\Objects",
        "the first spelling"
    );
    assert_eq!(
        tree.folder(objects).entry_count(),
        3,
        "every spelling counts"
    );
    assert_eq!(
        file_paths(&tree, &archive, objects),
        ["Data\\Objects\\a.cgf", "Data\\objects\\b.cgf"],
        "its own Entries in path order"
    );
    Ok(())
}

#[test]
fn a_folder_holds_its_subfolders_by_name_and_its_own_entries_in_path_order() -> TestResult {
    let (_folder, archive) = open(&[
        "Data/zeta.txt",
        "Data/Libs/a.xml",
        "Data/Game2.dcb",
        "Data/libs_old/b.xml",
        "readme.txt",
    ])?;
    let tree = EntryTree::build(&archive);
    let data = tree.find_folder("Data/").ok_or("no Data")?;
    assert_eq!(child_names(&tree, data), ["Libs", "libs_old"], "by name");
    assert_eq!(
        file_paths(&tree, &archive, data),
        ["Data/Game2.dcb", "Data/zeta.txt"],
        "only the Entries directly in it"
    );
    assert_eq!(tree.folder(data).path(), "Data", "the path as stored");
    assert_eq!(
        file_paths(&tree, &archive, tree.root()),
        ["readme.txt"],
        "the top level"
    );
    assert_eq!(tree.folder(tree.root()).entry_count(), 5, "every Entry");
    assert!(tree.find_folder("Data/Missing").is_none(), "unknown folder");
    assert!(tree.find_folder("Data/Game2.dcb").is_none(), "an Entry");
    Ok(())
}

#[test]
fn entry_ids_follow_path_order_ignoring_case_and_separator_kind() -> TestResult {
    let (_folder, archive) = open(&["b\\x", "A/y", "a\\z", "B/w"])?;
    let tree = EntryTree::build(&archive);
    let paths: Vec<&str> = (0..4)
        .filter_map(|id| tree.entry(&archive, id))
        .map(Entry::path)
        .collect();
    assert_eq!(paths, ["A/y", "a\\z", "B/w", "b\\x"], "path order");
    assert_eq!(tree.len(), 4, "every Entry");
    assert_eq!(
        tree.folder(tree.find_folder("a").ok_or("no a")?).entries(),
        0..2,
        "a folder is one id range"
    );
    Ok(())
}
