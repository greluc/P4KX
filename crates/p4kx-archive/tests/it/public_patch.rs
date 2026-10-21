//! The public patch of a Build, told by the RSI Launcher log line with the same change number.

use std::error::Error;
use std::path::Path;

use p4kx_archive::Build;
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

#[test]
fn a_build_has_no_public_patch_until_a_log_is_read() -> TestResult {
    let dir = tempfile::tempdir()?;

    assert_eq!(build(dir.path(), "11674325")?.public_patch(), None);
    Ok(())
}

#[test]
fn the_log_line_with_the_same_change_number_gives_the_public_patch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let logs = tempfile::tempdir()?;
    write_logs(
        logs.path(),
        &line("SC LIVE 4.7.1-live.11638371"),
        &(line("SC PTU 4.8.0-ptu.11811531") + &line("SC LIVE 4.7.2-live.11674325")),
    )?;

    let build = build(dir.path(), "11674325")?.with_public_patch_from(logs.path());

    assert_eq!(build.public_patch(), Some("4.7.2"));
    Ok(())
}

#[test]
fn the_old_log_serves_a_change_number_the_current_log_lacks() -> TestResult {
    let dir = tempfile::tempdir()?;
    let logs = tempfile::tempdir()?;
    write_logs(
        logs.path(),
        &line("SC LIVE 4.7.1-live.11638371"),
        &line("SC LIVE 4.8.1-live.11875683"),
    )?;

    let build = build(dir.path(), "11638371")?.with_public_patch_from(logs.path());

    assert_eq!(build.public_patch(), Some("4.7.1"));
    Ok(())
}

#[test]
fn a_two_part_version_of_a_tech_preview_is_kept_as_logged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let logs = tempfile::tempdir()?;
    write_logs(logs.path(), "", &line("SC TECH-PREVIEW 4.7-tp.11581647"))?;

    let build = build(dir.path(), "11581647")?.with_public_patch_from(logs.path());

    assert_eq!(build.public_patch(), Some("4.7"));
    Ok(())
}

#[test]
fn no_line_for_the_change_number_leaves_the_public_patch_absent() -> TestResult {
    let dir = tempfile::tempdir()?;
    let logs = tempfile::tempdir()?;
    write_logs(
        logs.path(),
        "",
        &(line("SC LIVE 4.8.2-live.12061511") + &line("SC LIVE 4.8.1-live.11875683")),
    )?;

    let build = build(dir.path(), "12913470")?.with_public_patch_from(logs.path());

    assert_eq!(
        build.public_patch(),
        None,
        "not guessed from the nearest line"
    );
    Ok(())
}

#[test]
fn a_change_number_that_is_only_part_of_another_does_not_match() -> TestResult {
    let dir = tempfile::tempdir()?;
    let logs = tempfile::tempdir()?;
    write_logs(logs.path(), "", &line("SC LIVE 4.8.2-live.112061511"))?;

    let build = build(dir.path(), "12061511")?.with_public_patch_from(logs.path());

    assert_eq!(build.public_patch(), None);
    Ok(())
}

#[test]
fn a_malformed_line_or_version_is_ignored() -> TestResult {
    let dir = tempfile::tempdir()?;
    let logs = tempfile::tempdir()?;
    let text = [
        "garbage 12345 (SC LIVE\n",
        "(SC LIVE latest-live.12345)\n",
        "(SC LIVE 4.7.-live.12345)\n",
        "(SC LIVE 4.7.2 live.12345)\n",
        "(SC )12345\n",
        "(SC LIVE 4.7.2-live12345)\n",
    ]
    .concat();
    write_logs(logs.path(), "", &text)?;

    let build = build(dir.path(), "12345")?.with_public_patch_from(logs.path());

    assert_eq!(build.public_patch(), None);
    Ok(())
}

#[test]
fn an_invalid_utf8_byte_in_the_log_does_not_hide_a_later_line() -> TestResult {
    let dir = tempfile::tempdir()?;
    let logs = tempfile::tempdir()?;
    let mut bytes = b"\xff\xfe broken\n".to_vec();
    bytes.extend(line("SC LIVE 4.7.2-live.11674325").into_bytes());
    std::fs::write(logs.path().join("log.log"), bytes)?;

    let build = build(dir.path(), "11674325")?.with_public_patch_from(logs.path());

    assert_eq!(build.public_patch(), Some("4.7.2"));
    Ok(())
}

#[test]
fn a_missing_log_folder_leaves_the_public_patch_absent() -> TestResult {
    let dir = tempfile::tempdir()?;

    let build = build(dir.path(), "11674325")?.with_public_patch_from(&dir.path().join("nowhere"));

    assert_eq!(build.public_patch(), None);
    Ok(())
}
