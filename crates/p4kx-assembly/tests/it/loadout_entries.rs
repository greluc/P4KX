//! The default Loadout as one entry per occupied port, shared by the exports.

use std::error::Error;

use p4kx_assembly::{LoadoutResolver, PortSource};
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
        entity(0x1, "Ship")
            .port(TestPort::new("empty_port"))
            .port(TestPort::new("default_port").default_item(0x5))
            .loadout(TestLoadout::Manual(vec![
                TestLoadoutEntry::class("turret", "Turret"),
                TestLoadoutEntry::class("lost", "NoSuchClass"),
                TestLoadoutEntry::empty("blank"),
            ])),
        entity(0x2, "Turret")
            .port(TestPort::new("gun"))
            .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "gun", "Gun",
            )])),
        entity(0x3, "Gun"),
        entity(0x5, "Fallback"),
    ]
}

#[test]
fn the_loadout_lists_every_occupied_port_by_path() -> TestResult {
    let bytes = fleet()
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let ship = resolver.entities().find("Ship")?;
    let tree = resolver.resolve(&ship)?;
    let entries = tree.loadout();
    let rows: Vec<_> = entries
        .iter()
        .map(|entry| {
            (
                entry.path(),
                entry.class_name(),
                entry.port().source(),
                entry.entity().is_some(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("turret", Some("Turret"), PortSource::Loadout, true),
            ("turret/gun", Some("Gun"), PortSource::Loadout, true),
            ("lost", Some("NoSuchClass"), PortSource::Loadout, false),
            (
                "default_port",
                Some("Fallback"),
                PortSource::PortDefault,
                true
            ),
        ],
        "empty ports are left out, an unresolved class name stays, a port default is listed"
    );
    assert!(
        entries
            .iter()
            .filter(|entry| entry.entity().is_some())
            .all(|entry| entry.guid().is_some()),
        "every attached Entity has its Record's GUID"
    );
    Ok(())
}
