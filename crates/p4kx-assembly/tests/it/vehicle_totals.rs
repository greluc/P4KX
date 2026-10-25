// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The sums over a vehicle implementation: mass, hull hit points, thresholds and value origins.

use crate::vehicle_support::{DEFINITION, TestResult, files};

use p4kx_assembly::{ValueOrigin, VehicleImplementation};

const PATCH: &str = "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Heavy.xml";

const XML: &str = r#"<Vehicle name="TEST_Ship">
  <Parts>
    <Part name="hull" class="Animated" mass="1000" damageMax="500" detachRatio="0.5">
      <DamageBehaviors>
        <DamageBehavior damageRatioMin="0.4"><Group name="Destroy"/></DamageBehavior>
      </DamageBehaviors>
      <Parts>
        <Part name="wing" id="wingId" class="Animated" mass="200" damageMax="100" detachRatio="0.25"/>
        <Part name="thruster" class="ItemPort" mass="50" damageMax="80x">
          <ItemPort flags="$uneditable"/>
        </Part>
        <Part name="odd" class="Animated" mass="97858&#x202C;.0d" damagemax="&#x200E;10f"/>
        <Part name="bad" class="Animated" mass="n/a" damageMax="12,5x"/>
        <Part name="blank" class="Animated" mass=" "/>
        <Part name="tag" class="Animated" mass="&#xE0020;5&#x110BD;&#xFEFF;"/>
      </Parts>
    </Part>
  </Parts>
  <Modifications>
    <Modification name="Heavy" patchFile="Modifications/Heavy">
      <Elems><Elem idRef="wingId" name="damageMax" value="150"/></Elems>
    </Modification>
  </Modifications>
</Vehicle>"#;

const HEAVY: &str = r#"<Modifications>
  <Part id="wingId" name="wing" class="Animated" mass="260" damageMax="120"/>
</Modifications>"#;

fn read(modification: Option<&str>) -> Result<VehicleImplementation, Box<dyn std::error::Error>> {
    let files = files(&[(DEFINITION, XML), (PATCH, HEAVY)]);
    Ok(VehicleImplementation::read(
        &files,
        "TEST_Ship",
        DEFINITION,
        modification,
    )?)
}

#[test]
fn the_base_sums_mass_and_hull_and_leaves_unreadable_values_out() -> TestResult {
    let totals = read(None)?.totals();
    assert!(
        (totals.mass() - 99_113.0).abs() < 1e-6,
        "1000 + 200 + 50 + 97858 + 5: {}",
        totals.mass()
    );
    assert_eq!(totals.mass_parts(), 5, "the blank mass states none");
    assert_eq!(totals.mass_origins().count(ValueOrigin::Base), 5);
    assert_eq!(
        totals.cleaned(),
        3,
        "the mass and the damageMax of odd, the mass of tag"
    );
    let unreadable: Vec<_> = totals
        .unreadable()
        .iter()
        .map(|value| (value.part(), value.attribute(), value.raw()))
        .collect();
    assert_eq!(
        unreadable,
        [("bad", "mass", "n/a")],
        "text that is no number is listed with its raw text, never counted as 0"
    );
    Ok(())
}

#[test]
fn the_hull_sums_the_damage_of_structural_parts_only() -> TestResult {
    let totals = read(None)?.totals();
    assert!(
        (totals.health() - (500.0 + 100.0 + 10.0 + 12.0)).abs() < 1e-6,
        "the uneditable thruster is left out; 12,5x reads as its prefix 12: {}",
        totals.health()
    );
    assert_eq!(totals.health_parts(), 4);
    let partial: Vec<_> = totals
        .partial()
        .iter()
        .map(|value| (value.part(), value.attribute(), value.raw(), value.read()))
        .collect();
    assert_eq!(
        partial,
        [("bad", "damageMax", "12,5x", 12.0)],
        "units d and f are no partial read; the left-out thruster's 80x is in no sum"
    );
    let destroy = totals.destruction();
    assert_eq!(destroy.len(), 1);
    assert_eq!(destroy[0].part(), "hull");
    assert!((destroy[0].damage() - 200.0).abs() < 1e-6, "0.4 of 500");
    let detach = totals.detach();
    assert_eq!(detach.len(), 1, "the hull destroys the ship instead");
    assert_eq!(detach[0].part(), "wing");
    assert!((detach[0].damage() - 25.0).abs() < 1e-6, "0.25 of 100");
    Ok(())
}

#[test]
fn the_modification_says_where_each_value_comes_from() -> TestResult {
    let implementation = read(Some("Heavy"))?;
    assert!(
        implementation.issues().is_empty(),
        "issues: {:?}",
        implementation.issues()
    );
    let wing = implementation
        .iter()
        .find(|part| part.name() == "wing")
        .ok_or("no wing")?;
    assert_eq!(wing.mass(), Some(260.0));
    assert_eq!(wing.origin_of("mass"), ValueOrigin::Patch);
    assert_eq!(wing.damage_max(), Some(150.0));
    assert_eq!(wing.origin_of("damageMax"), ValueOrigin::Elem);
    assert_eq!(wing.origin_of("detachRatio"), ValueOrigin::Base);
    let totals = implementation.totals();
    assert_eq!(totals.mass_origins().count(ValueOrigin::Patch), 1);
    assert_eq!(totals.mass_origins().count(ValueOrigin::Base), 4);
    assert_eq!(totals.health_origins().count(ValueOrigin::Elem), 1);
    assert_eq!(totals.health_origins().total(), 4);
    Ok(())
}

#[test]
fn a_value_unchanged_by_the_modification_stays_base() -> TestResult {
    let hull = read(Some("Heavy"))?
        .iter()
        .find(|part| part.name() == "hull")
        .map(|part| (part.origin_of("mass"), part.origin_of("damageMax")))
        .ok_or("no hull")?;
    assert_eq!(hull, (ValueOrigin::Base, ValueOrigin::Base));
    Ok(())
}
