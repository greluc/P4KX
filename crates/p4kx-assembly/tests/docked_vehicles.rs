//! Docked vehicles in a Loadout: marked, and left unexpanded when the resolver is told to stop.

use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, LoadoutResolver, PortNode};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestLoadout, TestLoadoutEntry, TestPort, entity_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

fn entity(guid: u128, class_name: &str) -> TestEntity {
    TestEntity::new(guid, class_name, format!("entities/{class_name}.xml"))
}

fn fleet() -> Vec<TestEntity> {
    vec![
        entity(0x1, "Carrier")
            .attach_type("NOITEM_Vehicle")
            .loadout(TestLoadout::Manual(vec![
                TestLoadoutEntry::class("hangar", "Snub"),
                TestLoadoutEntry::class("gun_port", "Gun"),
            ])),
        entity(0x2, "Snub")
            .attach_type("NOITEM_Vehicle_Snub")
            .port(TestPort::new("snub_gun"))
            .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "snub_gun", "SnubGun",
            )])),
        entity(0x3, "SnubGun").attach_type("WeaponGun"),
        entity(0x4, "Gun").attach_type("WeaponGun"),
    ]
}

fn resolve(stop: bool, class_name: &str) -> Result<EntityNode, Box<dyn Error>> {
    let bytes = fleet()
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    if stop {
        resolver = resolver.stopping_at_docked_vehicles();
    }
    let entity = resolver.entities().find(class_name)?;
    Ok(resolver.resolve(&entity)?)
}

fn attached<'n>(node: &'n EntityNode, port: &str) -> Result<&'n EntityNode, Box<dyn Error>> {
    let port: &PortNode = node
        .ports()
        .iter()
        .find(|candidate| candidate.name() == port)
        .ok_or("no such port")?;
    match port.attachment() {
        Attachment::Entity(child) => Ok(child),
        _ => Err("nothing attached".into()),
    }
}

#[test]
fn docked_vehicles_are_expanded_and_marked_by_default() -> TestResult {
    let tree = resolve(false, "Carrier")?;
    let snub = attached(&tree, "hangar")?;
    assert!(snub.is_docked_vehicle(), "marked");
    assert!(snub.is_expanded(), "expanded");
    assert_eq!(snub.ports().len(), 1, "the snub craft's gun port is built");
    assert!(!attached(&tree, "gun_port")?.is_docked_vehicle(), "a gun");
    Ok(())
}

#[test]
fn stopping_keeps_the_attachment_marked_without_its_subtree() -> TestResult {
    let tree = resolve(true, "Carrier")?;
    let snub = attached(&tree, "hangar")?;
    assert_eq!(snub.class_name(), "Snub", "still attached");
    assert!(snub.is_docked_vehicle(), "marked");
    assert!(!snub.is_expanded(), "not expanded");
    assert!(snub.ports().is_empty(), "no subtree");
    assert!(snub.issues().is_empty(), "no issues");
    let gun = attached(&tree, "gun_port")?;
    assert!(gun.is_expanded() && !gun.is_docked_vehicle(), "items stay");
    Ok(())
}

#[test]
fn the_resolved_entity_itself_is_expanded_even_if_it_is_a_vehicle() -> TestResult {
    let tree = resolve(true, "Snub")?;
    assert!(tree.is_docked_vehicle(), "marked");
    assert!(tree.is_expanded(), "the root is expanded");
    assert_eq!(attached(&tree, "snub_gun")?.class_name(), "SnubGun");
    Ok(())
}
