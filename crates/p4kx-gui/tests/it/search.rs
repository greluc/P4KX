//! Searching the whole Archive with a Filter, driven without drawing.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use p4kx_extraction::{Archive, Filter};
use p4kx_gui::{Browser, Language, Model, SortColumn};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const PATHS: [&str; 7] = [
    "Data\\Libs\\config.xml",
    "Data\\Libs\\Foundry\\Records\\ship.xml",
    "Data\\Textures\\hull.dds",
    "Data\\Game2.dcb",
    "Data\\ObjectContainers\\PU\\Aurora_MR.socpak",
    "Data\\Objects\\Ships\\RSI\\aurora\\hull.cgf",
    "readme.txt",
];

fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    PATHS
        .iter()
        .zip(1_usize..)
        .fold(ArchiveBuilder::new(), |builder, (path, size)| {
            builder.entry(TestEntry::stored(*path, vec![1; size * 10]))
        })
        .write_to(&path)?;
    Ok((folder, path))
}

fn opened(path: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.to_path_buf(), || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while !model.poll() {
        if Instant::now() > deadline {
            return Err("the Archive did not open in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(model)
}

fn searched(browser: &mut Browser, text: &str) -> TestResult {
    browser.search(text, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while browser.is_searching() {
        if Instant::now() > deadline {
            return Err(format!("searching {text:?} did not finish").into());
        }
        std::thread::sleep(Duration::from_millis(2));
        browser.poll();
    }
    Ok(())
}

fn shown_paths(browser: &Browser) -> Vec<&str> {
    browser
        .shown()
        .iter()
        .filter_map(|&id| browser.index().entry(id).map(p4kx_extraction::Entry::path))
        .collect()
}

#[test]
fn search_results_are_the_entries_p4kx_search_lists() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;

    for text in [
        "*.xml",
        "*.dds",
        "*.dcb",
        "Game2.dcb",
        "aurora",
        "data/libs",
        "",
        " ",
    ] {
        searched(browser, text)?;
        let selected = Filter::new(text).select(&archive);
        let expected: BTreeSet<&str> = selected.iter().map(|entry| entry.path()).collect();
        let shown: BTreeSet<&str> = shown_paths(browser).into_iter().collect();

        assert_eq!(shown, expected, "{text:?} matches as in p4kx search");
        assert_eq!(
            browser.shown().len(),
            selected.len(),
            "{text:?} shows each match once"
        );
    }
    Ok(())
}

#[test]
fn a_search_covers_the_whole_archive_whatever_folder_was_selected() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    let textures = browser
        .index()
        .find_folder("Data\\Textures")
        .ok_or("no Textures")?;
    browser.select_folder(textures);

    searched(browser, "AURORA")?;

    assert_eq!(
        shown_paths(browser),
        [
            "Data\\ObjectContainers\\PU\\Aurora_MR.socpak",
            "Data\\Objects\\Ships\\RSI\\aurora\\hull.cgf",
        ],
        "both, ignoring case, sorted by name"
    );
    assert_eq!(browser.search_text(), "AURORA", "the field keeps the text");
    Ok(())
}

#[test]
fn clearing_the_search_returns_to_the_selected_folder() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    let libs = browser.index().find_folder("Data\\Libs").ok_or("no Libs")?;
    browser.select_folder(libs);
    searched(browser, "*.dds")?;

    searched(browser, "")?;

    assert_eq!(
        shown_paths(browser),
        [
            "Data\\Libs\\config.xml",
            "Data\\Libs\\Foundry\\Records\\ship.xml"
        ],
        "Libs again"
    );
    Ok(())
}

#[test]
fn selecting_a_folder_ends_the_search() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    searched(browser, "*.xml")?;
    let textures = browser
        .index()
        .find_folder("Data\\Textures")
        .ok_or("no Textures")?;

    browser.select_folder(textures);

    assert_eq!(browser.search_text(), "", "the field is cleared");
    assert_eq!(
        shown_paths(browser),
        ["Data\\Textures\\hull.dds"],
        "Textures"
    );
    Ok(())
}

#[test]
fn a_newer_search_replaces_one_still_running() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;

    browser.search("*.xml", || {});
    searched(browser, "*.dds")?;

    assert_eq!(
        shown_paths(browser),
        ["Data\\Textures\\hull.dds"],
        "the latest"
    );
    Ok(())
}

#[test]
fn search_results_follow_the_sort() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    searched(browser, "*.xml")?;

    browser.sort_by(SortColumn::Size);
    browser.sort_by(SortColumn::Size);

    assert_eq!(
        shown_paths(browser),
        [
            "Data\\Libs\\Foundry\\Records\\ship.xml",
            "Data\\Libs\\config.xml"
        ],
        "largest first"
    );
    Ok(())
}

#[test]
fn selected_entries_outside_the_search_are_counted_apart() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    searched(browser, "*.xml")?;
    browser.set_all_shown_checked(true);

    searched(browser, "aurora")?;
    let first = *browser.shown().first().ok_or("nothing shown")?;
    browser.set_checked(first, true);

    assert_eq!(browser.checked_count(), 3, "two XML files and one aurora");
    assert_eq!(
        browser.hidden_checked_count(),
        2,
        "the XML files are hidden"
    );
    assert!(!browser.all_shown_checked(), "one aurora is unchecked");
    browser.set_all_shown_checked(true);
    assert!(browser.all_shown_checked(), "both aurora checked");
    assert_eq!(browser.checked_count(), 4, "four in all");
    Ok(())
}
