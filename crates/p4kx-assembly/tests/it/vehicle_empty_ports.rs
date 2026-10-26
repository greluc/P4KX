// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Ports only the vehicle implementation declares are listed empty and can take an Entity.

use crate::vehicle_support;

use p4kx_assembly::{
    Attachment, LoadoutChanges, LoadoutResolver, PortNode, PortSource, PortVisibility,
    ReplacementMismatch,
};
use p4kx_dataforge::DataForge;
use p4kx_testkit::TestEntity;
use vehicle_support::{DEFINITION, TestResult, database, files, resolve, standard_files};

fn port<'a>(ports: &'a [PortNode], name: &str) -> Result<&'a PortNode, String> {
    ports
        .iter()
        .find(|port| port.name() == name)
        .ok_or_else(|| format!("no port {name}"))
}

#[test]
fn a_port_only_the_vehicle_declares_is_listed_empty_after_the_defined_ports() -> TestResult {
    let tree = resolve(DEFINITION, "Open", Some(&standard_files()))?;

    let names: Vec<&str> = tree.ports().iter().map(PortNode::name).collect();
    assert_eq!(
        names,
        [
            "hardpoint_gun",
            "hardpoint_seat",
            "hardpoint_unnamed",
            "hardpoint_extra"
        ],
        "the Modification un-skips hardpoint_extra, which no Loadout or Record names"
    );
    let extra = port(tree.ports(), "hardpoint_extra")?;
    assert_eq!(extra.source(), PortSource::Empty);
    assert_eq!(extra.source().code(), "empty");
    assert!(extra.definition().is_none());
    assert!(matches!(extra.attachment(), Attachment::Empty));
    assert!(extra.origin().is_none());
    assert_eq!(extra.visibility(), PortVisibility::Invisible);
    Ok(())
}

#[test]
fn a_port_the_record_defines_is_not_listed_twice() -> TestResult {
    let tree = resolve(DEFINITION, "", Some(&standard_files()))?;

    let count = |name: &str| {
        tree.ports()
            .iter()
            .filter(|port| port.name().eq_ignore_ascii_case(name))
            .count()
    };
    assert_eq!(count("hardpoint_gun"), 1, "Hardpoint_Gun is the same port");
    assert_eq!(count("hardpoint_seat"), 1);
    assert!(
        tree.ports()
            .iter()
            .all(|port| port.source() != PortSource::Empty),
        "every vehicle port is defined by the Record here"
    );
    Ok(())
}

#[test]
fn a_replacement_attaches_an_entity_to_such_a_port() -> TestResult {
    let gun = TestEntity::new(0x2, "TEST_Gun", "entities/test_gun.xml");
    let bytes = database(DEFINITION, "Open").record(gun.record()).build();
    let forge = DataForge::open(&bytes)?;
    let files = standard_files();
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let ship = resolver.entities().find("TEST_Ship")?;
    let changes = LoadoutChanges::new().replace("hardpoint_extra", "TEST_Gun");

    let tree = resolver.resolve_with(&ship, &changes)?;

    let extra = port(tree.ports(), "hardpoint_extra")?;
    assert_eq!(extra.source(), PortSource::Replaced);
    assert!(matches!(extra.attachment(), Attachment::Entity(_)));
    Ok(())
}

const REPEATED: &str = r#"<Vehicle name="TEST_Ship"><Parts>
  <Part name="body" class="Animated"><Parts>
    <Part name="hardpoint_twin" class="ItemPort"><ItemPort minSize="1" maxSize="1" flags=""/></Part>
    <Part name="Hardpoint_Twin" class="ItemPort"><ItemPort minSize="2" maxSize="2" flags=""/></Part>
  </Parts></Part>
</Parts></Vehicle>"#;

#[test]
fn parts_that_share_a_name_are_listed_each_once() -> TestResult {
    let tree = resolve(DEFINITION, "", Some(&files(&[(DEFINITION, REPEATED)])))?;

    let twins: Vec<&str> = tree
        .ports()
        .iter()
        .filter(|port| port.source() == PortSource::Empty)
        .map(PortNode::name)
        .collect();
    assert_eq!(twins, ["hardpoint_twin", "Hardpoint_Twin"]);
    Ok(())
}

const SIZED: &str = r#"<Vehicle name="TEST_Ship"><Parts>
  <Part name="body" class="Animated">
    <Parts><Part name="hardpoint_small" class="ItemPort"><ItemPort minSize="1" maxSize="2" flags=""/></Part></Parts>
  </Part>
</Parts></Vehicle>"#;

fn replace_in_sized_port(
    size: i32,
) -> Result<Vec<ReplacementMismatch>, Box<dyn std::error::Error>> {
    let item = TestEntity::new(0x2, "TEST_Item", "entities/test_item.xml")
        .attach_type("WeaponGun")
        .size(size);
    let bytes = database(DEFINITION, "").record(item.record()).build();
    let forge = DataForge::open(&bytes)?;
    let files = files(&[(DEFINITION, SIZED)]);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let ship = resolver.entities().find("TEST_Ship")?;
    let changes = LoadoutChanges::new().replace("hardpoint_small", "TEST_Item");
    Ok(resolver
        .resolve_changed(&ship, &changes)?
        .mismatches()
        .to_vec())
}

#[test]
fn the_vehicle_sizes_of_such_a_port_decide_which_entity_fits() -> TestResult {
    let tree = resolve(DEFINITION, "", Some(&files(&[(DEFINITION, SIZED)])))?;
    let small = port(tree.ports(), "hardpoint_small")?;
    assert_eq!(small.source(), PortSource::Empty);
    let constraints = small.constraints().ok_or("the vehicle's sizes")?;
    assert_eq!(
        (constraints.min_size(), constraints.max_size()),
        (Some(1), Some(2))
    );

    assert!(replace_in_sized_port(2)?.is_empty(), "size 2 fits");
    assert_eq!(
        replace_in_sized_port(5)?.len(),
        1,
        "size 5 is applied and reported"
    );
    Ok(())
}
