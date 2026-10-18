//! The recently opened Archives the GUI keeps in the settings file.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_settings::{RecentArchives, Settings};

type TestResult = Result<(), Box<dyn Error>>;

fn paths(recent: &RecentArchives) -> Vec<&Path> {
    recent.iter().map(PathBuf::as_path).collect()
}

#[test]
fn the_latest_archive_comes_first_and_reopening_moves_it_up() {
    let mut recent = RecentArchives::default();

    recent.push(PathBuf::from(r"C:\SC\LIVE\Data.p4k"));
    recent.push(PathBuf::from(r"C:\SC\PTU\Data.p4k"));
    recent.push(PathBuf::from(r"C:\SC\LIVE\Data.p4k"));

    assert_eq!(
        paths(&recent),
        [
            Path::new(r"C:\SC\LIVE\Data.p4k"),
            Path::new(r"C:\SC\PTU\Data.p4k")
        ],
        "LIVE moved to the front, listed once"
    );
}

#[test]
fn only_the_latest_archives_are_kept() {
    let mut recent = RecentArchives::default();

    for number in 0..=RecentArchives::LIMIT {
        recent.push(PathBuf::from(format!("{number}.p4k")));
    }

    assert_eq!(recent.len(), RecentArchives::LIMIT, "capped");
    assert_eq!(
        recent.iter().next().map(PathBuf::as_path),
        Some(Path::new(&format!("{}.p4k", RecentArchives::LIMIT))),
        "newest first"
    );
    assert!(
        recent.iter().all(|path| path != Path::new("0.p4k")),
        "the oldest is dropped"
    );
}

#[test]
fn recent_archives_survive_a_round_trip_next_to_the_manual_entries() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let mut settings = Settings::default();
    settings.recent.push(PathBuf::from(r"C:\SC\LIVE\Data.p4k"));
    settings
        .channels
        .add_folder(PathBuf::from(r"D:\Games\StarCitizen"));

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn a_file_without_recent_archives_has_none() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "language = \"de-DE\"\n")?;

    assert!(
        Settings::load(&file)?.recent.is_empty(),
        "no recent Archives"
    );
    Ok(())
}
