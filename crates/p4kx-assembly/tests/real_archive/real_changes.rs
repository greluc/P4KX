// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Own configurations of the Gladius of a real Archive: ports declared only by its vehicle
//! implementation take their constraints from it.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, ChangeError, Incompatibility,
    LoadoutChanges, LoadoutResolver, Part, PortSource,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_shield_is_rejected_for_the_power_plant_by_the_vehicle_implementation() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find("AEGS_Gladius")?;

    let rejected = resolver.resolve_with(
        &record,
        &LoadoutChanges::new().replace("hardpoint_power_plant", "SHLD_GODI_S01_AllStop_SCItem"),
    );

    assert!(
        matches!(
            rejected,
            Err(ChangeError::Incompatible {
                reason: Incompatibility::Type { .. },
                ..
            })
        ),
        "{rejected:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_gladius_can_lose_its_guns_and_take_another_gun() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find("AEGS_Gladius")?;

    let changes = LoadoutChanges::new()
        .exclude("hardpoint_gun_nose")
        .exclude("hardpoint_gun_right_wing")
        .replace(
            "hardpoint_gun_left_wing/hardpoint_class_2",
            "GATS_BallisticGatling_S3",
        );
    let tree = resolver.resolve_with(&record, &changes)?;

    let ports = tree.walk();
    let source = |path: &str| {
        ports
            .iter()
            .find(|(found, _)| found == path)
            .map(|(_, port)| port.source())
    };
    assert_eq!(source("hardpoint_gun_nose"), Some(PortSource::Excluded));
    assert_eq!(
        source("hardpoint_gun_left_wing/hardpoint_class_2"),
        Some(PortSource::Replaced)
    );
    assert!(
        !ports
            .iter()
            .any(|(path, _)| path.starts_with("hardpoint_gun_nose/")),
        "nothing below an excluded port"
    );
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default().with_lod(2))?;
    let mut bytes = Vec::new();
    assembly.write_glb(&GlbOptions::default().with_lod(2), &mut bytes)?;
    check_glb(&bytes)?;
    assert!(
        assembly
            .parts()
            .iter()
            .all(|part| !part.path().starts_with("hardpoint_gun_nose")
                && !part.path().starts_with("hardpoint_gun_right_wing")),
        "no part of an excluded branch"
    );
    assert!(
        assembly
            .parts()
            .iter()
            .any(|part| part.class_name() == "GATS_BallisticGatling_S3"
                && part.path().starts_with("hardpoint_gun_left_wing")),
        "{:?}",
        assembly.parts().iter().map(Part::name).collect::<Vec<_>>()
    );
    Ok(())
}
