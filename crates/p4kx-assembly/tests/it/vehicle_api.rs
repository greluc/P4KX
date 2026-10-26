// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! What a vehicle implementation tells its callers: where each value comes from, the `Elem`
//! overrides that were applied, and the types and display name of an item port.

use crate::vehicle_support;

use p4kx_assembly::{ValueOrigin, VehicleImplementation};
use vehicle_support::{DEFINITION, TestResult, files, part_names};

const PATCH_PATH: &str = "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Wrapped.xml";

const BASE: &str = r#"<Vehicle name="TEST_Ship">
  <Parts>
    <Part name="body" class="Animated" mass="1500">
      <Parts>
        <Part name="Hardpoint_Gun" class="ItemPort" id="partGun" damageMax="120">
          <ItemPort id="portGun" minSize="1" maxSize="3" flags="invisible"
                    display_name="@vehicle_NameHardpointGun">
            <Types>
              <Type type="WeaponGun" subtypes="Gun, Rocket,,"/>
              <Type type="Turret"/>
            </Types>
          </ItemPort>
        </Part>
      </Parts>
    </Part>
  </Parts>
  <Modifications>
    <Modification name="Tuned">
      <Elems>
        <Elem idRef="portGun" name="flags" value=""/>
        <Elem idRef="partGun" name="damageMax" value="300"/>
        <Elem idRef="partGun" name="DAMAGEMAX" value="400"/>
        <Elem idRef="nobody" name="mass" value="1"/>
      </Elems>
    </Modification>
    <Modification name="Wrapped" patchFile="Modifications/Wrapped"/>
    <Modification name="Twice" patchFile="Modifications/Twice"/>
  </Modifications>
</Vehicle>"#;

const TWICE_PATH: &str = "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Twice.xml";

fn read(modification: Option<&str>) -> Result<VehicleImplementation, Box<dyn std::error::Error>> {
    let wrapped = r#"<Modifications><Parts><Part name="swapped" class="Animated" mass="5"/></Parts></Modifications>"#;
    let twice = r#"<Modifications>
      <Parts><Part name="first" class="Animated"/></Parts>
      <Parts><Part name="second" class="Animated"/></Parts>
    </Modifications>"#;
    let files = files(&[
        (DEFINITION, BASE),
        (PATCH_PATH, wrapped),
        (TWICE_PATH, twice),
    ]);
    Ok(VehicleImplementation::read(
        &files,
        "TEST_Ship",
        DEFINITION,
        modification,
    )?)
}

#[test]
fn an_item_port_gives_its_types_with_subtypes_and_its_display_name() -> TestResult {
    let vehicle = read(None)?;
    let port = vehicle.item_port("hardpoint_gun").ok_or("no port")?;
    assert_eq!(port.display_name(), Some("@vehicle_NameHardpointGun"));
    let kinds: Vec<(&str, Vec<&str>)> = port
        .item_types()
        .iter()
        .map(|kind| {
            (
                kind.item_type(),
                kind.subtypes().iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [("WeaponGun", vec!["Gun", "Rocket"]), ("Turret", vec![])],
        "a sub-type list is split at commas and trimmed"
    );
    assert_eq!(port.types(), ["WeaponGun", "Turret"]);
    Ok(())
}

#[test]
fn a_port_without_display_name_has_none() -> TestResult {
    let text = BASE.replace(r#"display_name="@vehicle_NameHardpointGun""#, "");
    let files = files(&[(DEFINITION, text.as_str())]);
    let vehicle = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, None)?;
    let port = vehicle.item_port("hardpoint_gun").ok_or("no port")?;
    assert_eq!(port.display_name(), None);
    Ok(())
}

#[test]
fn applied_elems_are_listed_with_the_last_one_standing_for_each_value() -> TestResult {
    let vehicle = read(Some("Tuned"))?;
    let applied: Vec<(&str, &str, &str)> = vehicle
        .elems()
        .iter()
        .map(|elem| (elem.id_ref(), elem.name(), elem.value()))
        .collect();
    assert_eq!(
        applied,
        [
            ("portGun", "flags", ""),
            ("partGun", "damageMax", "300"),
            ("partGun", "DAMAGEMAX", "400"),
        ],
        "an Elem whose idRef no element has is not applied"
    );
    let winner = vehicle
        .elem_overriding("partGun", "damagemax")
        .ok_or("no elem")?;
    assert_eq!(winner.value(), "400");
    assert!(vehicle.elem_overriding("nobody", "mass").is_none());
    assert_eq!(
        vehicle
            .modification_report()
            .map(|report| report.unmatched_elems().to_vec()),
        Some(vec!["nobody".to_owned()])
    );
    Ok(())
}

#[test]
fn no_modification_applies_no_elems() -> TestResult {
    assert_eq!(read(None)?.elems(), []);
    Ok(())
}

#[test]
fn each_value_names_the_file_it_came_from() -> TestResult {
    let tuned = read(Some("Tuned"))?;
    let gun = tuned.item_ports().next().ok_or("no port part")?;
    assert_eq!(gun.origin_of("damageMax"), ValueOrigin::Elem);
    assert_eq!(gun.origin_of("mass"), ValueOrigin::Base);
    assert_eq!(
        tuned.source_path(gun.origin_of("damageMax")),
        Some(DEFINITION)
    );
    let port = gun.item_port().ok_or("no item port")?;
    assert_eq!(port.origin_of("flags"), ValueOrigin::Elem);
    assert_eq!(port.origin_of("minSize"), ValueOrigin::Base);
    assert_eq!(tuned.source_path(ValueOrigin::Patch), None);

    let wrapped = read(Some("Wrapped"))?;
    let swapped = wrapped.iter().next().ok_or("no part")?;
    assert_eq!(swapped.origin_of("mass"), ValueOrigin::Patch);
    assert_eq!(
        wrapped.source_path(swapped.origin_of("mass")),
        Some(PATCH_PATH)
    );
    Ok(())
}

#[test]
fn several_children_of_a_name_in_a_patch_all_replace_the_base_child_of_that_name() -> TestResult {
    let vehicle = read(Some("Twice"))?;
    assert!(vehicle.issues().is_empty(), "{:?}", vehicle.issues());
    assert_eq!(part_names(vehicle.iter()), ["first", "second"]);
    let report = vehicle.modification_report().ok_or("no report")?;
    assert_eq!(report.unmatched_children(), [] as [String; 0]);
    Ok(())
}
