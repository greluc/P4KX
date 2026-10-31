// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity loadout --loadout-pick` chooses the alternative of a group of a generated Loadout.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry,
    assortment_record, entity_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

const ASSORTMENT: u128 = 0x100;

fn body(class: &str) -> TestLoadout {
    TestLoadout::Manual(vec![TestLoadoutEntry::class("Body_ItemPort", class)])
}

/// An archive whose `Beast` has a generated Loadout with a `Body` group of two alternatives.
fn archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let builder = [
        TestEntity::new(1, "Beast", "entities/beast.xml")
            .loadout(TestLoadout::RandomGenerated(vec![ASSORTMENT])),
        TestEntity::new(2, "Body_Red", "entities/body_red.xml"),
        TestEntity::new(3, "Body_Blue", "entities/body_blue.xml"),
    ]
    .iter()
    .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
        builder.record(entity.record())
    });
    let dcb = builder
        .record(assortment_record(
            ASSORTMENT,
            "beast",
            &[("Body", vec![body("Body_Red"), body("Body_Blue")])],
        ))
        .build();
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", dcb))
        .write_to(&path)?;
    Ok(path)
}

struct Run {
    stdout: String,
    code: Option<i32>,
}

fn loadout(archive: &Path, picks: &[&str]) -> Result<Run, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    command
        .args(["entity", "loadout"])
        .arg(archive)
        .arg("Beast");
    for pick in picks {
        command.args(["--loadout-pick", pick]);
    }
    let output = command.output()?;
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?,
        code: output.status.code(),
    })
}

#[test]
fn without_a_pick_the_first_alternative_is_attached() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = loadout(&archive(dir.path())?, &[])?;

    assert_eq!(run.code, Some(0));
    assert!(run.stdout.contains("Body_Red"), "{}", run.stdout);
    assert!(!run.stdout.contains("Body_Blue"), "{}", run.stdout);
    Ok(())
}

#[test]
fn a_named_pick_attaches_the_other_alternative() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = loadout(&archive(dir.path())?, &["body=1"])?;

    assert_eq!(run.code, Some(0));
    assert!(run.stdout.contains("Body_Blue"), "{}", run.stdout);
    assert!(!run.stdout.contains("Body_Red"), "{}", run.stdout);
    Ok(())
}

#[test]
fn an_index_beyond_the_group_is_listed_and_the_first_stays() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = loadout(&archive(dir.path())?, &["Body=7"])?;

    assert_eq!(run.code, Some(0));
    assert!(run.stdout.contains("! "), "{}", run.stdout);
    assert!(run.stdout.contains("beyond"), "{}", run.stdout);
    assert!(run.stdout.contains("Body_Red"), "{}", run.stdout);
    Ok(())
}

#[test]
fn a_malformed_pick_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = loadout(&archive(dir.path())?, &["Body=first"])?;

    assert_eq!(run.code, Some(2));
    Ok(())
}
