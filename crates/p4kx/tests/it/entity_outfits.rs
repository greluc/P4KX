// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity loadout` and `p4kx armour glb --loadout` over the synthetic NPC actor with two
//! outfits: the one without name tags is worn, the tagged one is not.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, NPC_CLASS, npc_entries};

type TestResult = Result<(), Box<dyn Error>>;

fn archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    npc_entries()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

#[test]
fn the_text_lists_the_outfits_and_whether_each_is_worn() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;

    let run = p4kx_testkit::p4kx()?
        .args(["entity", "loadout"])
        .arg(&archive)
        .arg(NPC_CLASS)
        .output()?;

    let stdout = String::from_utf8(run.stdout)?;
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8(run.stderr)?
    );
    assert!(
        stdout.contains("outfit 1 worn, no name tags: -\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("outfit 2 not worn, name tags tagged: -\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Clothing_Torso_0: TEST_NpcShirt"),
        "{stdout}"
    );
    Ok(())
}

#[test]
fn the_json_has_the_outfits_and_an_entity_without_outfit_has_none() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;
    let json = |entity: &str| -> Result<serde_json::Value, Box<dyn Error>> {
        let run = p4kx_testkit::p4kx()?
            .arg("--json")
            .args(["entity", "loadout"])
            .arg(&archive)
            .arg(entity)
            .output()?;
        assert_eq!(
            run.status.code(),
            Some(0),
            "{}",
            String::from_utf8(run.stderr)?
        );
        Ok(serde_json::from_slice(&run.stdout)?)
    };

    let npc = json(NPC_CLASS)?;
    let outfits = npc["outfits"]["outfits"].as_array().ok_or("outfits")?;
    assert_eq!(outfits.len(), 2);
    assert_eq!(outfits[0]["applied"], true);
    assert_eq!(outfits[0]["tags"], serde_json::json!([]));
    assert_eq!(outfits[1]["applied"], false);
    assert_eq!(outfits[1]["tags"], serde_json::json!(["tagged"]));
    assert!(json("TEST_NpcBody")?.get("outfits").is_none());
    Ok(())
}

#[test]
fn armour_glb_reports_the_outfits_under_the_loadout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;

    let run = p4kx_testkit::p4kx()?
        .arg("--json")
        .args(["armour", "glb"])
        .arg(&archive)
        .args(["--loadout", NPC_CLASS, "--out"])
        .arg(dir.path().join("npc.glb"))
        .output()?;

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8(run.stderr)?
    );
    let json: serde_json::Value = serde_json::from_slice(&run.stdout)?;
    assert_eq!(json["loadout"]["outfits"]["outfits"][1]["applied"], false);
    Ok(())
}
