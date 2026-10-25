// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Blender packages of a real Archive: documented ships, an Interior and a Character are
//! packaged through the CLI, validate against the schema and stay within the documented size.

#![expect(clippy::print_stdout, reason = "the measurements are the output")]

use std::ffi::OsStr;
use std::io::Read;

use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

use super::package_support::{Built, Measured, TestResult, build, nodes_of, verify, workspace};

const SHIPS: [&str; 4] = [
    "AEGS_Gladius",
    "AEGS_Avenger_Titan",
    "RSI_Polaris",
    "RSI_Constellation_Andromeda",
];
const INTERIOR_SHIPS: [&str; 2] = ["RSI_Constellation_Andromeda", "DRAK_Cutlass_Black"];
const CHF: &str = "masculinedefault.chf";
const LOADOUT: &str =
    "Scripts/Loadouts/Character/Human/Armor/NPC/Criminal/Pyro/Criminal_Gang_Guard_Heavy_Boss.xml";

const MAX_SHIP_BYTES: u64 = 2 << 30;
const MAX_COARSE_SHIP_BYTES: u64 = 128 << 20;
const MAX_INTERIOR_BYTES: u64 = 3 << 29;
const MAX_CHARACTER_BYTES: u64 = 128 << 20;
const MAX_MESH_RATIO: f64 = 0.85;
const MIN_SHARING: f64 = 0.3;

fn shared(built: &Built, measured: &Measured) {
    let drawn = (built.report.get("drawn_nodes")).and_then(Value::as_u64);
    let drawn = drawn.unwrap_or_default();
    assert_eq!(
        measured.instances, drawn,
        "{}: every drawing node is an instance of one mesh",
        built.name
    );
    assert!(
        measured.sharing() >= MIN_SHARING,
        "{}: only {:.3} of the drawing nodes share a mesh",
        built.name,
        measured.sharing()
    );
    assert!(
        measured.mesh_ratio() <= MAX_MESH_RATIO,
        "{}: the meshes take {:.3} of their undeduplicated size",
        built.name,
        measured.mesh_ratio()
    );
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn documented_ships_are_packages_within_the_documented_size() -> TestResult {
    for class in SHIPS {
        let built = build("entity", Some(class), &class.to_lowercase(), &[])?;
        let measured = verify(&built)?;
        shared(&built, &measured);
        assert!(measured.meshes > 1, "{class}: {} meshes", measured.meshes);
        assert!(
            measured.instances > measured.meshes as u64,
            "{class}: {} instances of {} meshes, nothing is shared",
            measured.instances,
            measured.meshes
        );
        assert!(
            measured.bytes <= MAX_SHIP_BYTES,
            "{class}: {} bytes",
            measured.bytes
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_coarser_texture_mip_makes_a_ship_package_much_smaller() -> TestResult {
    let built = build(
        "entity",
        Some("AEGS_Gladius"),
        "aegs_gladius-mip2",
        &[OsStr::new("--texture-mip"), OsStr::new("2")],
    )?;
    let measured = verify(&built)?;
    shared(&built, &measured);
    assert!(
        measured.bytes <= MAX_COARSE_SHIP_BYTES,
        "{} bytes",
        measured.bytes
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn ships_with_an_interior_are_packages_with_containers_objects_and_lights() -> TestResult {
    for class in INTERIOR_SHIPS {
        let name = format!("{}-interior", class.to_lowercase());
        let built = build("entity", Some(class), &name, &[OsStr::new("--interior")])?;
        let measured = verify(&built)?;
        shared(&built, &measured);
        let containers = nodes_of(&built.manifest, "container")?;
        let objects = nodes_of(&built.manifest, "object")?;
        let lights = nodes_of(&built.manifest, "light")?;
        println!("INTERIOR {class}: {containers} containers, {objects} objects, {lights} lights");
        assert!(
            containers > 1 && objects > 0,
            "{class}: {containers}/{objects}"
        );
        assert_eq!(built.report["containers"], containers, "{class}");
        assert_eq!(built.report["objects"], objects, "{class}");
        assert_eq!(built.report["lights"], lights, "{class}");
        assert!(
            measured.bytes <= MAX_INTERIOR_BYTES,
            "{class}: {} bytes",
            measured.bytes
        );
    }
    Ok(())
}

fn extract_chf(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path().to_ascii_lowercase().ends_with(name))
        .ok_or_else(|| format!("the Archive holds no {name}"))?;
    let mut bytes = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut bytes)?;
    let file = workspace()?.join(name);
    std::fs::write(&file, bytes)?;
    Ok(file)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_character_with_armour_is_one_skinned_package() -> TestResult {
    let chf = extract_chf(CHF)?;
    let built = build(
        "character",
        None,
        "character",
        &[
            OsStr::new("--chf"),
            chf.as_os_str(),
            OsStr::new("--loadout"),
            OsStr::new(LOADOUT),
            OsStr::new("--anim"),
            OsStr::new("ai_civ_stand_ntrl_tray_2h_carry_idle_01"),
            OsStr::new("--anim"),
            OsStr::new("ai_cryopod_sleep_idle_01"),
        ],
    )?;
    let measured = verify(&built)?;
    assert_eq!(measured.meshes, 1, "one skinned GLB");
    assert_eq!(nodes_of(&built.manifest, "character")?, 1);
    assert_eq!(built.manifest["textures"].as_array().map(Vec::len), Some(0));
    let rig = &built.manifest["meshes"][0]["rig"];
    let count = |key: &str| rig[key].as_u64().unwrap_or_default();
    println!(
        "CHARACTER: {} bones, {} skinned meshes, {} morph targets, {} parts, {} animations",
        count("bones"),
        count("skinned_meshes"),
        count("morph_targets"),
        rig["parts"].as_array().map_or(0, Vec::len),
        rig["animations"].as_array().map_or(0, Vec::len)
    );
    assert!(count("bones") > 50, "{} bones", count("bones"));
    assert!(
        count("skinned_meshes") > 1,
        "{} skinned meshes",
        count("skinned_meshes")
    );
    assert!(count("morph_targets") > 0, "the head has morph targets");
    assert_eq!(
        rig["animations"].as_array().map(Vec::len),
        Some(2),
        "both animations"
    );
    assert_eq!(rig["parts"][0]["origin"], "body", "the body comes first");
    assert!(
        rig["parts"].as_array().map_or(0, Vec::len) > 1,
        "armour is worn"
    );
    assert!(
        measured.bytes <= MAX_CHARACTER_BYTES,
        "{} bytes",
        measured.bytes
    );
    Ok(())
}
