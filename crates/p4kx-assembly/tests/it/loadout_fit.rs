// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! What a port takes, for pickers and for ports only the vehicle implementation declares.

use crate::loadout_changes::{applied, database, entities, held, port};
use crate::vehicle_support;

use std::error::Error;

use p4kx_assembly::{ChangeError, Incompatibility, LoadoutChanges, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{TestEntity, TestLoadout, TestLoadoutEntry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_constraints_and_the_check_are_public_for_pickers() -> TestResult {
    let tree = applied(&LoadoutChanges::new())?;
    let bytes = database(&entities()).build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let s2 = resolver.entities().find("GUN_S2")?;
    let s3 = resolver.entities().find("GUN_S3")?;
    let (s2, s3) = (resolver.item_facts(&s2)?, resolver.item_facts(&s3)?);

    let weapon = port(&tree, "weapon_a")?;
    let constraints = weapon.constraints().ok_or("no constraints")?;

    assert_eq!(
        (constraints.min_size(), constraints.max_size()),
        (Some(1), Some(2))
    );
    assert_eq!(constraints.types()[0].name(), "WeaponGun");
    assert_eq!(constraints.types()[0].sub_types(), ["Gun"]);
    assert!(weapon.accepts(&s2).is_ok());
    assert_eq!(
        weapon.accepts(&s3),
        Err(Incompatibility::TooLarge { size: 3, max: 2 })
    );
    assert_eq!(
        (s2.item_type(), s2.sub_type(), s2.size()),
        ("WeaponGun", "Gun", Some(2))
    );
    Ok(())
}

#[test]
fn ports_of_the_vehicle_implementation_are_checked_against_its_item_ports() -> TestResult {
    let frame = r#"<Vehicle name="TEST_Frame"><Parts><Part name="body" class="Animated"><Parts>
        <Part name="hardpoint_gun" class="ItemPort">
          <ItemPort minSize="2" maxSize="3"><Types><Type type="WeaponGun"/></Types></ItemPort>
        </Part>
    </Parts></Part></Parts></Vehicle>"#;
    let definition = "Scripts/Entities/Vehicles/Implementations/Xml/TEST_Frame.xml";
    let files = vehicle_support::files(&[(definition, frame)]);
    let frame_entity = TestEntity::new(0x30, "TEST_Frame", "entities/test_frame.xml")
        .vehicle(definition, "")
        .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
            "hardpoint_gun",
            "GUN_S2",
        )]));
    let mut all = entities();
    all.push(frame_entity);
    let bytes = database(&all).build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let frame = resolver.entities().find("TEST_Frame")?;

    let small = resolver.resolve_with(
        &frame,
        &LoadoutChanges::new().replace("hardpoint_gun", "GUN_S1"),
    );
    let shield = resolver.resolve_with(
        &frame,
        &LoadoutChanges::new().replace("hardpoint_gun", "SHIELD_S1"),
    );
    let fitting = resolver.resolve_with(
        &frame,
        &LoadoutChanges::new().replace("hardpoint_gun", "GUN_S3"),
    )?;

    assert!(
        matches!(
            small,
            Err(ChangeError::Incompatible {
                reason: Incompatibility::TooSmall { size: 1, min: 2 },
                ..
            })
        ),
        "{small:?}"
    );
    assert!(
        matches!(
            shield,
            Err(ChangeError::Incompatible {
                reason: Incompatibility::Type { .. },
                ..
            })
        ),
        "{shield:?}"
    );
    assert_eq!(held(&fitting, "hardpoint_gun")?, Some("GUN_S3"));
    let constraints = port(&fitting, "hardpoint_gun")?
        .constraints()
        .ok_or("the vehicle's item port gives constraints")?;
    assert_eq!(constraints.types()[0].name(), "WeaponGun");
    Ok(())
}
