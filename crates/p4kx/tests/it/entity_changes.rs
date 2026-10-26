// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity loadout` and `p4kx entity glb` with `--exclude` and `--replace`.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestEntry, assembly_entries, check_glb, sample_ship};

type TestResult = Result<(), Box<dyn Error>>;

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

fn write_archive(dir: &Path, entries: Vec<TestEntry>) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

fn sample_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    write_archive(
        dir,
        vec![TestEntry::zstd("Data\\Game2.dcb", sample_ship().build())],
    )
}

fn run(
    archive: &Path,
    action: &str,
    entity: &str,
    extra: &[&str],
    json: bool,
) -> Result<Run, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    let output = command
        .args(["entity", action])
        .arg(archive)
        .arg(entity)
        .args(extra)
        .output()?;
    let archive_text = archive.display().to_string();
    let json_text = serde_json::to_string(&archive_text)?;
    let json_text = json_text.trim_matches('"');
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?
            .replace(json_text, "<archive>")
            .replace(&archive_text, "<archive>"),
        stderr: String::from_utf8(output.stderr)?,
        code: output.status.code(),
    })
}

#[test]
fn an_excluded_branch_is_marked_and_its_ports_are_gone() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = sample_archive(dir.path())?;

    let run = run(
        &archive,
        "loadout",
        "TEST_Ship",
        &[
            "--exclude",
            "hardpoint_weapon_nose",
            "--exclude",
            "HARDPOINT_DASHBOARD",
        ],
        false,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    insta::assert_snapshot!(run.stdout);
    Ok(())
}

#[test]
fn a_replacement_is_marked_and_resolved_with_its_own_loadout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = sample_archive(dir.path())?;

    let run = run(
        &archive,
        "loadout",
        "TEST_Ship",
        &[
            "--replace",
            "hardpoint_relay=TEST_Dashboard",
            "--replace",
            "hardpoint_seat_pilot=TEST_Screen",
        ],
        false,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    insta::assert_snapshot!(run.stdout);
    Ok(())
}

#[test]
fn the_json_tree_names_the_source_of_a_changed_port_and_counts_them() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = sample_archive(dir.path())?;

    let run = run(
        &archive,
        "loadout",
        "TEST_Ship",
        &[
            "--exclude",
            "hardpoint_cargo",
            "--replace",
            "hardpoint_relay=TEST_Seat",
        ],
        true,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    let source = |name: &str| {
        value["entity"]["ports"]
            .as_array()
            .and_then(|ports| ports.iter().find(|port| port["name"] == name))
            .map(|port| port["source"].as_str().unwrap_or_default().to_owned())
    };
    assert_eq!(source("hardpoint_cargo").as_deref(), Some("excluded"));
    assert_eq!(source("hardpoint_relay").as_deref(), Some("replaced"));
    assert_eq!(
        (&value["excluded"], &value["replaced"]),
        (&1.into(), &1.into())
    );
    Ok(())
}

#[test]
fn an_entity_that_does_not_fit_the_port_is_attached_with_a_warning() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = sample_archive(dir.path())?;

    let run = run(
        &archive,
        "loadout",
        "TEST_Ship",
        &["--replace", "hardpoint_weapon_nose=TEST_Seat"],
        false,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("TEST_Seat"), "{}", run.stdout);
    assert!(
        run.stderr
            .contains("warning: TEST_Seat does not fit the port hardpoint_weapon_nose: its item type is none, the port takes WeaponGun"),
        "{}",
        run.stderr
    );
    Ok(())
}

#[test]
fn the_json_lists_an_entity_that_does_not_fit_the_port() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = sample_archive(dir.path())?;

    let run = run(
        &archive,
        "loadout",
        "TEST_Ship",
        &["--replace", "hardpoint_weapon_nose=TEST_Seat"],
        true,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let json: serde_json::Value = serde_json::from_str(&run.stdout)?;
    let listed = json["incompatible"]
        .as_array()
        .ok_or("no incompatible list")?;
    assert_eq!(listed.len(), 1, "{json}");
    assert_eq!(listed[0]["port"], "hardpoint_weapon_nose");
    assert_eq!(listed[0]["entity"], "TEST_Seat");
    assert_eq!(listed[0]["kind"], "item_type");
    Ok(())
}

#[test]
fn bad_changes_are_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = sample_archive(dir.path())?;
    let cases: [(&[&str], &str); 5] = [
        (&["--exclude", "no_such_port"], "has no port no_such_port"),
        (&["--replace", "hardpoint_relay"], "<port>=<entity>"),
        (
            &["--replace", "hardpoint_relay=NO_SUCH_ENTITY"],
            "NO_SUCH_ENTITY",
        ),
        (
            &[
                "--exclude",
                "hardpoint_relay",
                "--replace",
                "hardpoint_relay=TEST_Seat",
            ],
            "contradict",
        ),
        (&["--exclude", "a//b"], "no port path"),
    ];

    for (extra, expected) in cases {
        let run = run(&archive, "loadout", "TEST_Ship", extra, false)?;

        assert_eq!(run.code, Some(2), "{extra:?}: {}", run.stderr);
        assert!(run.stderr.contains(expected), "{extra:?}: {}", run.stderr);
    }
    Ok(())
}

fn glb(archive: &Path, out: &Path, extra: &[&str], json: bool) -> Result<Run, Box<dyn Error>> {
    let mut all = vec!["--out", out.to_str().ok_or("path")?];
    all.extend_from_slice(extra);
    run(archive, "glb", "TEST_Ship", &all, json)
}

#[test]
fn the_glb_leaves_out_excluded_parts_and_takes_replacements() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), assembly_entries())?;
    let out = dir.path().join("ship.glb");

    let run = glb(
        &archive,
        &out,
        &[
            "--exclude",
            "hardpoint_weapon_nose/hardpoint_class_2",
            "--exclude",
            "hardpoint_door",
            "--replace",
            "hardpoint_relay=TEST_Mount",
        ],
        false,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let checked = check_glb(&std::fs::read(&out)?)?;
    let named = |name: &str| {
        checked
            .document
            .nodes()
            .any(|node| node.name() == Some(name))
    };
    assert!(named("hardpoint_weapon_nose/TEST_Mount"), "the mount stays");
    assert!(!named("hardpoint_class_2/TEST_Gun"), "its gun is excluded");
    assert!(
        !named("hardpoint_relay/TEST_Relay"),
        "the relay is replaced"
    );
    assert!(named("hardpoint_relay/TEST_Mount"), "by a mount");
    assert!(
        !run.stdout.contains("skipped hardpoint_door"),
        "an excluded part is not an issue: {}",
        run.stdout
    );
    for line in [
        "excluded hardpoint_weapon_nose/hardpoint_class_2",
        "excluded hardpoint_door",
        "replaced hardpoint_relay by TEST_Mount",
        "4 parts (3 with models)",
    ] {
        assert!(run.stdout.contains(line), "{line}: {}", run.stdout);
    }
    Ok(())
}

#[test]
fn the_glb_json_report_lists_the_changes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), assembly_entries())?;
    let out = dir.path().join("ship.glb");

    let run = glb(
        &archive,
        &out,
        &[
            "--exclude",
            "hardpoint_door",
            "--replace",
            "hardpoint_relay=TEST_Mount",
        ],
        true,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(value["excluded"], serde_json::json!(["hardpoint_door"]));
    assert_eq!(
        value["replaced"],
        serde_json::json!([{"port": "hardpoint_relay", "entity": "TEST_Mount"}])
    );
    Ok(())
}

#[test]
fn a_rejected_change_writes_no_glb() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), assembly_entries())?;
    let out = dir.path().join("ship.glb");

    let run = glb(&archive, &out, &["--exclude", "no_such_port"], false)?;

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(!out.exists(), "nothing written");
    Ok(())
}
