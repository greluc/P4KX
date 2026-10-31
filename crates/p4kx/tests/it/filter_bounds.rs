// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Filter text that is too long or has too many wildcards is invalid input in every command that
//! takes a Filter.

use std::error::Error;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestContainer, TestEntry, TestRecord, TestStruct, TestType,
    TestValue, p4kx,
};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

/// Writes an Archive that holds a DataForge and a socpak container, for the commands that read
/// either before they look at their Filter.
fn forge_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let forge = DataForgeBuilder::new()
        .structure(TestStruct::new("Gear").field("name", TestType::String))
        .record(
            TestRecord::new(1, "Gear", "Gear.Gladius", "libs/ships/gladius.xml")
                .value("name", TestValue::text("Gladius")),
        )
        .build();
    let container = TestContainer::new().entry(TestEntry::stored("room\\good.xml", "<Good/>"));
    let path = dir.join("forge.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge))
        .entry(TestEntry::stored("Data\\room.socpak", container.build()))
        .write_to(&path)?;
    Ok(path)
}

/// Commands that take Filter text: the words of the command, its arguments in which `@archive`,
/// `@dir` and `@out` stand for an Archive, an existing folder and a new path, and the option that
/// takes the Filter.
const FILTER_COMMANDS: [(&[&str], &[&str], &str); 16] = [
    (&["grep"], &["@archive", "gladius"], "--filter"),
    (&["grep"], &["@archive", "gladius"], "--exclude"),
    (&["verify"], &["@archive"], "--filter"),
    (&["verify"], &["@archive"], "--exclude"),
    (&["compare"], &["@archive", "@archive"], "--filter"),
    (&["catalog"], &["@archive", "--out", "@out"], "--filter"),
    (&["ships"], &["@archive", "--out", "@out"], "--filter"),
    (
        &["dcb", "export"],
        &["@archive", "--out", "@out"],
        "--filter",
    ),
    (&["dcb", "fields"], &["@archive"], "--path"),
    (&["convert"], &["@dir", "--out", "@out"], "--filter"),
    (&["convert"], &["@dir", "--out", "@out"], "--exclude"),
    (
        &["socpak", "verify"],
        &["@archive", "Data\\room.socpak"],
        "--filter",
    ),
    (
        &["socpak", "verify"],
        &["@archive", "Data\\room.socpak"],
        "--exclude",
    ),
    (
        &["socpak", "extract"],
        &["@archive", "Data\\room.socpak", "--out", "@out"],
        "--filter",
    ),
    (
        &["socpak", "extract"],
        &["@archive", "Data\\room.socpak", "--out", "@out"],
        "--exclude",
    ),
    (&["extract"], &["@archive", "--out", "@out"], "--filter"),
];

fn filter_command(
    (words, arguments, option): (&[&str], &[&str], &str),
    (archive, dir): (&Path, &Path),
    text: &str,
) -> Result<Command, Box<dyn Error>> {
    let mut run = p4kx()?;
    run.args(words);
    for argument in arguments {
        match *argument {
            "@archive" => run.arg(archive),
            "@dir" => run.arg(dir),
            "@out" => run.arg(dir.join("out")),
            other => run.arg(other),
        };
    }
    run.args([option, text]);
    Ok(run)
}

#[test]
fn filter_text_that_is_too_long_or_too_wild_is_invalid_input_everywhere() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = forge_archive(dir.path())?;
    let long = "a".repeat(2000);
    let wild = "*a".repeat(40);

    for case in FILTER_COMMANDS {
        for (text, message) in [(&long, "2000 bytes long"), (&wild, "40 wildcards")] {
            filter_command(case, (&archive, dir.path()), text)?
                .assert()
                .code(2)
                .stderr(predicate::str::contains(message));
        }
    }
    Ok(())
}
