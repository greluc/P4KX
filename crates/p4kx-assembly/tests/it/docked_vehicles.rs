// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Docked vehicles in a Loadout: marked, and left unexpanded when the resolver is told to stop.

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{
    Attachment, EntityNode, LoadoutFileError, LoadoutFiles, LoadoutResolver, PortNode,
};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestLoadout, TestLoadoutEntry, TestPort, entity_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

const SNUB_XML: &str = "Scripts/Entities/Vehicles/Implementations/Xml/Snub.xml";

struct CountingFiles {
    reads: AtomicUsize,
}

impl LoadoutFiles for CountingFiles {
    fn read(&self, path: &str) -> Result<Vec<u8>, LoadoutFileError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if path.eq_ignore_ascii_case(SNUB_XML) {
            Ok(br#"<Vehicle name="Snub"><Parts/></Vehicle>"#.to_vec())
        } else {
            Err(LoadoutFileError::NotFound)
        }
    }
}

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
            .vehicle(
                "Scripts/Entities/Vehicles/Implementations/Xml/Missing.xml",
                "Mod",
            )
            .port(TestPort::new("snub_gun"))
            .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "snub_gun", "SnubGun",
            )])),
        entity(0x3, "SnubGun").attach_type("WeaponGun"),
        entity(0x4, "Gun").attach_type("WeaponGun"),
    ]
}

fn resolve(
    stop: bool,
    class_name: &str,
    files: &CountingFiles,
) -> Result<EntityNode, Box<dyn Error>> {
    let bytes = fleet()
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge).with_files(files);
    if stop {
        resolver = resolver.stopping_at_docked_vehicles();
    }
    let entity = resolver.entities().find(class_name)?;
    Ok(resolver.resolve(&entity)?)
}

const fn counting() -> CountingFiles {
    CountingFiles {
        reads: AtomicUsize::new(0),
    }
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
    let files = counting();
    let tree = resolve(false, "Carrier", &files)?;
    let snub = attached(&tree, "hangar")?;
    assert!(snub.is_docked_vehicle(), "marked");
    assert!(snub.is_expanded(), "expanded");
    assert_eq!(snub.ports().len(), 1, "the snub craft's gun port is built");
    assert!(
        files.reads.load(Ordering::Relaxed) > 0,
        "its vehicle XML is looked up"
    );
    assert!(
        !snub.issues().is_empty(),
        "the missing vehicle XML is an issue"
    );
    assert!(!attached(&tree, "gun_port")?.is_docked_vehicle(), "a gun");
    Ok(())
}

#[test]
fn stopping_keeps_the_attachment_marked_without_its_subtree() -> TestResult {
    let files = counting();
    let tree = resolve(true, "Carrier", &files)?;
    let snub = attached(&tree, "hangar")?;
    assert_eq!(snub.class_name(), "Snub", "still attached");
    assert!(snub.is_docked_vehicle(), "marked");
    assert!(!snub.is_expanded(), "not expanded");
    assert!(snub.ports().is_empty(), "no subtree");
    assert!(snub.issues().is_empty(), "no issues");
    assert!(snub.vehicle().is_none(), "no vehicle implementation");
    assert!(snub.landing_gear().is_empty(), "no landing gear");
    assert_eq!(
        files.reads.load(Ordering::Relaxed),
        0,
        "no vehicle XML is read"
    );
    let gun = attached(&tree, "gun_port")?;
    assert!(gun.is_expanded() && !gun.is_docked_vehicle(), "items stay");
    Ok(())
}

#[test]
fn the_resolved_entity_itself_is_expanded_even_if_it_is_a_vehicle() -> TestResult {
    let tree = resolve(true, "Snub", &counting())?;
    assert!(!tree.is_docked_vehicle(), "the root is never marked");
    assert!(tree.is_expanded(), "the root is expanded");
    assert_eq!(attached(&tree, "snub_gun")?.class_name(), "SnubGun");
    Ok(())
}

fn records_read(stop: bool, attach_type: &str) -> Result<(usize, bool), Box<dyn Error>> {
    let fleet = [
        entity(0x1, "Carrier").loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("hangar", "Snub").loadout(TestLoadout::Manual(vec![
                TestLoadoutEntry::class("snub_gun", "NestedGun"),
            ])),
            TestLoadoutEntry::class("pod", "Pod").loadout(TestLoadout::Manual(vec![
                TestLoadoutEntry::class("pod_gun", "PodGun"),
            ])),
        ])),
        entity(0x2, "Snub").attach_type(attach_type),
        entity(0x3, "NestedGun").attach_type("WeaponGun"),
        entity(0x4, "Pod").attach_type("Misc"),
        entity(0x5, "PodGun").attach_type("WeaponGun"),
    ];
    let bytes = fleet
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
    let carrier = resolver.entities().find("Carrier")?;
    let tree = resolver.resolve(&carrier)?;
    let docked = attached(&tree, "hangar")?.is_docked_vehicle();
    assert_eq!(resolver.serial_reads(), 0, "every Record is prefetched");
    Ok((resolver.records_read(), docked))
}

#[test]
fn the_marker_ignores_the_case_of_the_item_type() -> TestResult {
    for attach_type in ["noitem_vehicle_snub", "NoItem_VEHICLE", "NOITEM_Vehicle"] {
        assert!(records_read(false, attach_type)?.1, "{attach_type}");
    }
    assert!(!records_read(false, "NOITEM_Vehicl")?.1, "too short");
    assert!(!records_read(false, "Item_Vehicle")?.1, "other prefix");
    Ok(())
}

#[test]
fn records_below_a_docked_vehicle_are_not_read_ahead_when_stopping() -> TestResult {
    let (expanded, _) = records_read(false, "NOITEM_Vehicle_Snub")?;
    let (stopped, docked) = records_read(true, "NOITEM_Vehicle_Snub")?;
    assert!(docked);
    assert_eq!(stopped + 1, expanded, "only the nested gun is skipped");
    Ok(())
}
