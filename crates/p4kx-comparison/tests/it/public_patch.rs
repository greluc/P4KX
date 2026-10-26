// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The public patch of both Builds in the Comparison report.

use std::error::Error;
use std::path::Path;

use p4kx_channels::PublicPatches;
use p4kx_comparison::{Archive, CompareOptions, Levels, compare};
use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const BUILD_ID: &str = "2cae0dfc-a941-4dcc-b82a-543b08b52d97";

fn open(root: &Path, channel: &str, change: &str) -> Result<Archive, Box<dyn Error>> {
    let folder = root.join(channel);
    std::fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", change.as_bytes().to_vec()))
        .write_to(&path)?;
    write_build_manifest(&folder, "sc-alpha-4.7.0", "4.7.0.1", change, BUILD_ID)?;
    Ok(Archive::open_with_key(
        path,
        Some(p4kx_testkit::test_key()),
    )?)
}

fn report(patches: Option<&PublicPatches>) -> Result<Value, Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let old = open(root.path(), "LIVE", "11638371")?;
    let new = open(root.path(), "PTU", "11674325")?;
    let options = CompareOptions {
        levels: Levels::ENTRIES,
        public_patches: patches,
        ..CompareOptions::default()
    };
    let comparison = compare(&old, &new, None, &options)?;
    Ok(serde_json::to_value(&comparison)?)
}

#[test]
fn the_report_states_the_public_patch_of_each_build() -> TestResult {
    let logs = tempfile::tempdir()?;
    std::fs::write(
        logs.path().join("log.log"),
        "(SC LIVE 4.7.1-live.11638371)\n(SC PTU 4.7.2-ptu.11674325)\n",
    )?;
    let patches = PublicPatches::read_from(logs.path());

    let report = report(Some(&patches))?;

    assert_eq!(report["source"]["old"]["build"]["public_patch"], "4.7.1");
    assert_eq!(report["source"]["new"]["build"]["public_patch"], "4.7.2");
    Ok(())
}

#[test]
fn the_report_states_null_without_public_patches() -> TestResult {
    let report = report(None)?;

    assert_eq!(
        report["source"]["old"]["build"]["public_patch"],
        Value::Null
    );
    assert_eq!(
        report["source"]["new"]["build"]["public_patch"],
        Value::Null
    );
    Ok(())
}
