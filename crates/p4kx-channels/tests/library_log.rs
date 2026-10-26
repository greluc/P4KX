// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Library folders and public patches in launcher logs of unusual size.

use std::error::Error;
use std::io::Write;
use std::path::Path;

use p4kx_channels::{PublicPatches, SearchPaths, detect, known_library_folders};
use p4kx_testkit::write_build_manifest;

type TestResult = Result<(), Box<dyn Error>>;

const FILLER_LINE: usize = 1024;

fn launch_line(channel_folder: &Path, padding: usize) -> Result<String, Box<dyn Error>> {
    let escaped = serde_json::to_string(&channel_folder.display().to_string())?;
    let inner = escaped.trim_matches('"');
    Ok(format!(
        "{{ \"msg\": \"{}Launching Star Citizen LIVE from ({inner})\" }}\n",
        " ".repeat(padding)
    ))
}

fn install(root: &Path) -> Result<SearchPaths, Box<dyn Error>> {
    let channel = root.join("Library").join("LIVE");
    std::fs::create_dir_all(&channel)?;
    std::fs::write(channel.join("Data.p4k"), b"")?;
    write_build_manifest(&channel, "sc-alpha-4.7.0", "4.7.0.1", "11674325", "id")?;
    let logs = root.join("rsilauncher").join("logs");
    std::fs::create_dir_all(&logs)?;
    Ok(SearchPaths {
        launcher: Some(root.join("rsilauncher")),
        ..SearchPaths::default()
    })
}

fn write_filler(file: &mut std::fs::File, lines: usize) -> TestResult {
    let filler = format!("{}\n", "x".repeat(FILLER_LINE - 1));
    for _ in 0..lines {
        file.write_all(filler.as_bytes())?;
    }
    Ok(())
}

fn assert_found(paths: &SearchPaths, root: &Path) {
    let library = root.join("Library");
    assert_eq!(detect(paths).len(), 1, "detected");
    assert!(
        known_library_folders(paths).contains(&library),
        "known to the guard"
    );
}

#[test]
fn a_library_line_at_the_start_of_a_log_over_16_mib_is_still_found() -> TestResult {
    let root = tempfile::tempdir()?;
    let paths = install(root.path())?;
    let log = root.path().join("rsilauncher").join("logs").join("log.log");
    let mut file = std::fs::File::create(log)?;
    file.write_all(launch_line(&root.path().join("Library").join("LIVE"), 0)?.as_bytes())?;
    write_filler(&mut file, 17 << 10)?;
    drop(file);

    assert_found(&paths, root.path());
    Ok(())
}

#[test]
fn a_library_line_over_64_kib_is_still_found() -> TestResult {
    let root = tempfile::tempdir()?;
    let paths = install(root.path())?;
    let log = root.path().join("rsilauncher").join("logs").join("log.log");
    std::fs::write(
        log,
        launch_line(&root.path().join("Library").join("LIVE"), 100 << 10)?,
    )?;

    assert_found(&paths, root.path());
    Ok(())
}

#[test]
fn a_tail_that_starts_on_a_line_start_keeps_that_line() -> TestResult {
    let logs = tempfile::tempdir()?;
    let mut text = "(SC LIVE 4.7.2-live.11674325)".to_owned();
    text.push_str(&" ".repeat(FILLER_LINE - 1 - text.len()));
    text.push('\n');
    let mut file = std::fs::File::create(logs.path().join("log.log"))?;
    write_filler(&mut file, 1 << 10)?;
    file.write_all(text.as_bytes())?;
    write_filler(&mut file, (16 << 10) - 1)?;
    drop(file);

    let patches = PublicPatches::read_from(logs.path());

    assert_eq!(patches.get("11674325"), Some("4.7.2"));
    Ok(())
}

#[test]
fn only_the_most_recent_changes_are_kept_at_the_cap() -> TestResult {
    let logs = tempfile::tempdir()?;
    let text: String = (1..=10_000_u32)
        .map(|change| format!("(SC LIVE 4.7.2-live.{change})\n"))
        .collect();
    std::fs::write(logs.path().join("log.log"), text)?;

    let patches = PublicPatches::read_from(logs.path());

    assert_eq!(patches.get("10000"), Some("4.7.2"), "the newest");
    assert_eq!(patches.get("1"), None, "the oldest");
    Ok(())
}
