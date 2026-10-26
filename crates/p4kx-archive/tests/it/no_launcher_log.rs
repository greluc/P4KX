// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Opening an Archive reads no RSI Launcher log: its Build never has a public patch.

use std::error::Error;
use std::path::Path;
use std::process::Command;

use p4kx_archive::Archive;
use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};

type TestResult = Result<(), Box<dyn Error>>;

const CHANGE: &str = "11674325";
const ARCHIVE_VARIABLE: &str = "P4KX_NO_LOG_ARCHIVE";
const CHILD: &str = "no_launcher_log::an_opened_archive_has_no_public_patch";

#[test]
fn opening_an_archive_ignores_a_launcher_log_naming_its_change_number() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .write_to(&archive)?;
    write_build_manifest(
        dir.path(),
        "sc-alpha-4.7.0",
        "4.7.0.1",
        CHANGE,
        "2cae0dfc-a941-4dcc-b82a-543b08b52d97",
    )?;
    let logs = dir.path().join("logs");
    std::fs::create_dir(&logs)?;
    let line = format!("{{ \"msg\": \"Delta update applied (SC LIVE 4.7.2-live.{CHANGE})\" }}\n");
    std::fs::write(logs.join("log.log"), line)?;

    let output = Command::new(std::env::current_exe()?)
        .args([CHILD, "--exact", "--test-threads", "1"])
        .env(ARCHIVE_VARIABLE, &archive)
        .env("P4KX_LAUNCHER_LOGS", &logs)
        .env("APPDATA", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "the child failed: {stdout}");
    assert!(stdout.contains("1 passed"), "the child ran: {stdout}");
    Ok(())
}

#[test]
fn an_opened_archive_has_no_public_patch() -> TestResult {
    let Some(archive) = std::env::var_os(ARCHIVE_VARIABLE) else {
        return Ok(());
    };
    let archive = Archive::open(Path::new(&archive))?;

    let build = archive.build().ok_or("the Build is known")?;
    assert_eq!(build.change_number(), CHANGE);
    assert_eq!(build.public_patch(), None);
    Ok(())
}
