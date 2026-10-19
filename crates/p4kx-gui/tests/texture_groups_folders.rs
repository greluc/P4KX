//! Split textures whose parts sit in a folder of another spelling than their header.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{Browser, ExtractionScope, Language, Model};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn opened() -> Result<(TempDir, Model), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", vec![1; 5]))
        .entry(TestEntry::stored("Data\\tex\\wall.dds", vec![2; 100]))
        .entry(TestEntry::stored("Data\\tex\\wall.dds.1", vec![3; 20]))
        .entry(TestEntry::stored("data\\tex\\WALL.DDS.2a", vec![4; 4]))
        .write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err("the Archive did not open in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok((folder, model))
}

fn browser(model: &mut Model) -> Result<&mut Browser, Box<dyn Error>> {
    Ok(model.browser_mut().ok_or("no Archive")?)
}

fn select(browser: &mut Browser, path: &str) -> TestResult {
    let folder = browser
        .index()
        .find_folder(path)
        .ok_or_else(|| format!("no folder {path}"))?;
    browser.select_folder(folder);
    Ok(())
}

#[test]
fn the_folder_of_the_header_counts_a_part_from_another_folder() -> TestResult {
    let (_folder, mut model) = opened()?;
    let browser = browser(&mut model)?;

    select(browser, "Data\\tex")?;

    assert_eq!(browser.shown().len(), 1, "the texture is one row");
    assert_eq!(browser.view_len(), 3, "the status bar counts Entries");
    assert_eq!(browser.scope_len(ExtractionScope::Shown), 3);
    assert_eq!(
        browser.scope_entries(ExtractionScope::Shown).len(),
        browser.scope_len(ExtractionScope::Shown),
        "the count and the Entries agree"
    );
    assert!(!browser.all_shown_checked());
    browser.set_all_shown_checked(true);
    assert!(browser.all_shown_checked(), "every Entry of the view");
    assert_eq!(browser.checked_count(), 3);
    assert_eq!(browser.hidden_checked_count(), 0);
    Ok(())
}

#[test]
fn the_folder_of_the_part_does_not_show_or_count_it() -> TestResult {
    let (_folder, mut model) = opened()?;
    let browser = browser(&mut model)?;
    select(browser, "Data\\tex")?;
    browser.set_all_shown_checked(true);

    select(browser, "data\\tex")?;

    assert!(browser.shown().is_empty(), "the part is folded away");
    assert_eq!(browser.scope_len(ExtractionScope::Shown), 0);
    assert_eq!(browser.scope_entries(ExtractionScope::Shown).len(), 0);
    assert!(
        !browser.all_shown_checked(),
        "nothing shown, nothing checked"
    );
    assert_eq!(browser.hidden_checked_count(), 3, "all three are elsewhere");
    Ok(())
}
