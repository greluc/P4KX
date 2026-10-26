// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The public patch of a Build, told by the RSI Launcher log line with the same change number.

use std::error::Error;
use std::io::Write;
use std::path::Path;

use p4kx_archive::Build;
use p4kx_channels::{PublicPatches, SearchPaths, detect, detect_with_patches};
use p4kx_testkit::write_build_manifest;

type TestResult = Result<(), Box<dyn Error>>;

const BUILD_ID: &str = "2cae0dfc-a941-4dcc-b82a-543b08b52d97";

fn build(folder: &Path, change_number: &str) -> Result<Build, Box<dyn Error>> {
    write_build_manifest(folder, "sc-alpha-4.7.0", "4.7.0.1", change_number, BUILD_ID)?;
    Ok(Build::read_from(folder)?.ok_or("a Build")?)
}

fn line(label: &str) -> String {
    format!(
        "{{ \"t\":\"2026-03-05 17:41:35.580\", \"[main][info] \": \"[Installer] Delta update applied ({label}) in C:\\\\Games\\\\StarCitizen\\\\LIVE\"  }},\n"
    )
}

fn write_logs(folder: &Path, old: &str, current: &str) -> TestResult {
    std::fs::write(folder.join("log.old.log"), old)?;
    std::fs::write(folder.join("log.log"), current)?;
    Ok(())
}

fn patch_for(logs: &Path, change: &str) -> Result<Option<String>, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let build = build(dir.path(), change)?;
    Ok(PublicPatches::read_from(logs)
        .attach(&build)
        .public_patch()
        .map(str::to_owned))
}

#[test]
fn a_build_has_no_public_patch_until_one_is_attached() -> TestResult {
    let dir = tempfile::tempdir()?;

    assert_eq!(build(dir.path(), "11674325")?.public_patch(), None);
    Ok(())
}

#[test]
fn the_log_line_with_the_same_change_number_gives_the_public_patch() -> TestResult {
    let logs = tempfile::tempdir()?;
    write_logs(
        logs.path(),
        &line("SC LIVE 4.7.1-live.11638371"),
        &(line("SC PTU 4.8.0-ptu.11811531") + &line("SC LIVE 4.7.2-live.11674325")),
    )?;

    assert_eq!(
        patch_for(logs.path(), "11674325")?.as_deref(),
        Some("4.7.2")
    );
    Ok(())
}

#[test]
fn the_newest_line_wins_and_the_current_log_is_newer_than_the_old_one() -> TestResult {
    let logs = tempfile::tempdir()?;
    write_logs(
        logs.path(),
        &line("SC LIVE 4.7.0-live.11674325"),
        &(line("SC LIVE 4.7.1-live.11674325") + &line("SC LIVE 4.7.2-live.11674325")),
    )?;

    assert_eq!(
        patch_for(logs.path(), "11674325")?.as_deref(),
        Some("4.7.2")
    );
    Ok(())
}

#[test]
fn the_old_log_serves_a_change_number_the_current_log_lacks() -> TestResult {
    let logs = tempfile::tempdir()?;
    write_logs(
        logs.path(),
        &line("SC LIVE 4.7.1-live.11638371"),
        &line("SC LIVE 4.8.1-live.11875683"),
    )?;

    assert_eq!(
        patch_for(logs.path(), "11638371")?.as_deref(),
        Some("4.7.1")
    );
    Ok(())
}

#[test]
fn a_two_part_version_of_a_tech_preview_is_kept_as_logged() -> TestResult {
    let logs = tempfile::tempdir()?;
    write_logs(logs.path(), "", &line("SC TECH-PREVIEW 4.7-tp.11581647"))?;

    assert_eq!(patch_for(logs.path(), "11581647")?.as_deref(), Some("4.7"));
    Ok(())
}

#[test]
fn no_line_for_the_change_number_leaves_the_public_patch_absent() -> TestResult {
    let logs = tempfile::tempdir()?;
    write_logs(
        logs.path(),
        "",
        &(line("SC LIVE 4.8.2-live.12061511") + &line("SC LIVE 4.8.1-live.11875683")),
    )?;

    assert_eq!(
        patch_for(logs.path(), "12913470")?,
        None,
        "not guessed from the nearest line"
    );
    Ok(())
}

#[test]
fn a_change_number_that_is_only_part_of_another_does_not_match() -> TestResult {
    let logs = tempfile::tempdir()?;
    write_logs(logs.path(), "", &line("SC LIVE 4.8.2-live.112061511"))?;

    assert_eq!(patch_for(logs.path(), "12061511")?, None);
    Ok(())
}

#[test]
fn a_malformed_line_or_version_is_ignored() -> TestResult {
    let logs = tempfile::tempdir()?;
    let text = [
        "garbage 12345 (SC LIVE\n",
        "(SC LIVE latest-live.12345)\n",
        "(SC LIVE 4.7.-live.12345)\n",
        "(SC LIVE 4..7-live.12345)\n",
        "(SC LIVE .7-live.12345)\n",
        "(SC LIVE 4.7.2 live.12345)\n",
        "(SC )12345\n",
        "(SC LIVE 4.7.2-live12345)\n",
        "(SC LIVE 4.7.2-live.12x45)\n",
        "(SC LIVE 4.7.2-live.)\n",
    ]
    .concat();
    write_logs(logs.path(), "", &text)?;

    assert_eq!(patch_for(logs.path(), "12345")?, None);
    Ok(())
}

#[test]
fn an_invalid_utf8_byte_in_the_log_does_not_hide_a_later_line() -> TestResult {
    let logs = tempfile::tempdir()?;
    let mut bytes = b"\xff\xfe broken\n".to_vec();
    bytes.extend(line("SC LIVE 4.7.2-live.11674325").into_bytes());
    std::fs::write(logs.path().join("log.log"), bytes)?;

    assert_eq!(
        patch_for(logs.path(), "11674325")?.as_deref(),
        Some("4.7.2")
    );
    Ok(())
}

#[test]
fn a_missing_log_folder_leaves_the_public_patch_absent() -> TestResult {
    let dir = tempfile::tempdir()?;

    assert_eq!(patch_for(&dir.path().join("nowhere"), "11674325")?, None);
    Ok(())
}

#[test]
fn a_folder_named_like_a_log_file_is_not_read() -> TestResult {
    let logs = tempfile::tempdir()?;
    std::fs::create_dir(logs.path().join("log.log"))?;
    std::fs::write(
        logs.path().join("log.old.log"),
        line("SC LIVE 4.7.2-live.11674325"),
    )?;

    assert_eq!(
        patch_for(logs.path(), "11674325")?.as_deref(),
        Some("4.7.2")
    );
    Ok(())
}

#[test]
fn a_line_over_the_bound_is_skipped_and_the_next_line_still_read() -> TestResult {
    let logs = tempfile::tempdir()?;
    let mut text = line("SC LIVE 4.7.9-live.11674325").trim_end().to_owned();
    text.push_str(&" ".repeat(200 << 10));
    text.push('\n');
    text.push_str(&line("SC LIVE 4.7.2-live.11638371"));
    write_logs(logs.path(), "", &text)?;

    assert_eq!(patch_for(logs.path(), "11674325")?, None, "the long line");
    assert_eq!(
        patch_for(logs.path(), "11638371")?.as_deref(),
        Some("4.7.2"),
        "the line after it"
    );
    Ok(())
}

#[test]
fn an_oversized_log_is_read_from_its_tail() -> TestResult {
    let logs = tempfile::tempdir()?;
    let mut file = std::fs::File::create(logs.path().join("log.log"))?;
    file.write_all(line("SC LIVE 4.7.1-live.11638371").as_bytes())?;
    let filler = format!("{}\n", "x".repeat(1023));
    for _ in 0..(17 << 10) {
        file.write_all(filler.as_bytes())?;
    }
    file.write_all(line("SC LIVE 4.7.2-live.11674325").as_bytes())?;
    drop(file);

    assert_eq!(
        patch_for(logs.path(), "11638371")?,
        None,
        "the head is beyond the bound"
    );
    assert_eq!(
        patch_for(logs.path(), "11674325")?.as_deref(),
        Some("4.7.2")
    );
    Ok(())
}

#[test]
fn detection_gives_each_channel_its_patch_from_one_read_of_the_launcher_log() -> TestResult {
    let root = tempfile::tempdir()?;
    let library = root.path().join("StarCitizen");
    let channel = library.join("LIVE");
    std::fs::create_dir_all(&channel)?;
    std::fs::write(channel.join("Data.p4k"), b"")?;
    build(&channel, "11674325")?;
    let launcher = root.path().join("rsilauncher");
    std::fs::create_dir_all(launcher.join("logs"))?;
    std::fs::write(
        launcher.join("logs").join("log.log"),
        line("SC LIVE 4.7.2-live.11674325"),
    )?;
    let paths = SearchPaths {
        default_library: Some(library),
        launcher: Some(launcher),
        ..SearchPaths::default()
    };

    let channels = detect(&paths);
    let (again, patches) = detect_with_patches(&paths);

    assert_eq!(channels.len(), 1);
    let found = channels[0].build().ok_or("a Build")?;
    assert_eq!(found.public_patch(), Some("4.7.2"));
    assert_eq!(again.len(), 1);
    assert_eq!(patches.get("11674325"), Some("4.7.2"));
    Ok(())
}

#[test]
fn detection_without_a_launcher_folder_reads_no_log() -> TestResult {
    let root = tempfile::tempdir()?;
    let library = root.path().join("StarCitizen");
    let channel = library.join("LIVE");
    std::fs::create_dir_all(&channel)?;
    std::fs::write(channel.join("Data.p4k"), b"")?;
    build(&channel, "11674325")?;

    let channels = detect(&SearchPaths {
        default_library: Some(library),
        ..SearchPaths::default()
    });

    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].build().ok_or("a Build")?.public_patch(), None);
    Ok(())
}
