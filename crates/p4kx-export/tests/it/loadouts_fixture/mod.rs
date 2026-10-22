//! A synthetic Archive with ships, a turret with ports, a docked vehicle, a leftover item and
//! vehicle implementations, for the loadouts of the catalog.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry,
    TestPort, TestRecord, TestStruct, TestType, TestValue, entity_schema,
};

pub(crate) const ALPHA: u128 = 0x10;
pub(crate) const SNUB: u128 = 0x11;
pub(crate) const BROKEN: u128 = 0x12;
pub(crate) const TURRET: u128 = 0x20;
pub(crate) const RACK: u128 = 0x21;
pub(crate) const GUN: u128 = 0x22;
pub(crate) const GUN_DEFAULT: u128 = 0x23;
pub(crate) const GUN_OVERRIDE: u128 = 0x24;
pub(crate) const GUN_TEMPLATE: u128 = 0x25;
pub(crate) const EMPTY_LOADOUT: u128 = 0x26;
pub(crate) const MISSING: u128 = 0x99;
pub(crate) const TAG_MOUNT: u128 = 0x40;
pub(crate) const TAG_GUN: u128 = 0x41;
pub(crate) const TAG_TURRET: u128 = 0x42;
const TAG_ALPHA: u128 = 0x43;
const TAG_BETA: u128 = 0x44;
const TAG_DUP_ALPHA: u128 = 0x45;
const TAG_DUP_BETA: u128 = 0x46;
const TAG_DATABASE: u128 = 0x47;
const TAG_PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

const SHIPS: &str = "libs/foundry/records/entities/spaceships";
const ITEMS: &str = "libs/foundry/records/entities/scitem/ships";
const XML_DIR: &str = "Data\\Scripts\\Entities\\Vehicles\\Implementations\\Xml\\";
const DEFINITION_DIR: &str = "Scripts/Entities/Vehicles/Implementations/Xml";

const ALPHA_XML: &str = r#"<Vehicle name="ACME_Alpha">
  <Parts>
    <Part name="hull" class="Animated">
      <Parts>
        <Part name="hardpoint_gun" class="ItemPort">
          <ItemPort minSize="1" maxSize="3" flags="$uneditable invisible" portTags="$Gun" requiredTags="Mount Unknown Dup">
            <Types><Type type="WeaponGun"/></Types>
          </ItemPort>
        </Part>
        <Part name="hardpoint_turret" class="ItemPort">
          <ItemPort minSize="9" maxSize="9" flags="invisible"/>
        </Part>
        <Part name="hardpoint_empty" class="ItemPort">
          <ItemPort minSize="2" maxSize="2">
            <Types><Type type="Missile"/><Type type="Bomb"/></Types>
          </ItemPort>
        </Part>
      </Parts>
    </Part>
  </Parts>
</Vehicle>"#;

fn definition(name: &str) -> String {
    format!("{DEFINITION_DIR}/{name}.xml")
}

fn ship(guid: u128, class: &str) -> TestEntity {
    TestEntity::new(guid, class, format!("{SHIPS}/{}.xml", class.to_lowercase()))
}

fn item(guid: u128, class: &str) -> TestEntity {
    TestEntity::new(guid, class, format!("{ITEMS}/{}.xml", class.to_lowercase()))
        .attach_type("WeaponGun")
}

fn gun_port(name: &str) -> TestPort {
    TestPort::new(name)
        .sizes(1, 2)
        .item_type("WeaponGun", ["Gun"])
        .default_item(GUN_DEFAULT)
}

fn alpha_loadout() -> TestLoadout {
    TestLoadout::Manual(vec![
        TestLoadoutEntry::class("hardpoint_turret", "ACME_Turret").loadout(TestLoadout::Manual(
            vec![TestLoadoutEntry::class("gun_1", "ACME_Gun_Override")],
        )),
        TestLoadoutEntry::class("hardpoint_gun", "ACME_Gun"),
        TestLoadoutEntry::class("hangar", "ACME_Snub"),
        TestLoadoutEntry::class("lost", "NoSuchClass"),
        TestLoadoutEntry::class("hardpoint_old", "ACME_Gun_template"),
        TestLoadoutEntry::reference("hardpoint_ref", MISSING),
    ])
}

fn entities() -> Vec<TestEntity> {
    vec![
        ship(ALPHA, "ACME_Alpha")
            .vehicle(definition("ACME_Alpha"), "")
            .port(
                TestPort::new("hardpoint_turret")
                    .sizes(2, 4)
                    .item_type("Turret", ["GunTurret"])
                    .flags("uneditable")
                    .tags("Turret", "Mount"),
            )
            .loadout(alpha_loadout()),
        ship(SNUB, "ACME_Snub")
            .attach_type("NOITEM_Vehicle_Snub")
            .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "snub_gun", "ACME_Gun",
            )])),
        ship(BROKEN, "ACME_Broken").vehicle(definition("ACME_Broken"), ""),
        item(TURRET, "ACME_Turret")
            .port(gun_port("gun_1"))
            .port(gun_port("gun_2")),
        item(RACK, "ACME_Rack").port(
            TestPort::new("slot_1")
                .sizes(1, 1)
                .item_type("Missile", Vec::<String>::new()),
        ),
        item(GUN, "ACME_Gun").tags("Gun Unknown Dup mount"),
        item(GUN_DEFAULT, "ACME_Gun_Default"),
        item(GUN_OVERRIDE, "ACME_Gun_Override"),
        item(GUN_TEMPLATE, "ACME_Gun_template"),
        item(EMPTY_LOADOUT, "ACME_Empty_Loadout").loadout(TestLoadout::Manual(Vec::new())),
    ]
}

fn references(guids: &[u128]) -> TestValue {
    TestValue::Array(guids.iter().map(|g| TestValue::Reference(*g)).collect())
}

fn tag(guid: u128, name: &str, children: &[u128]) -> TestRecord {
    TestRecord::new(guid, "Tag", format!("Tag.{name}"), TAG_PATH)
        .value("tagName", TestValue::text(name))
        .value("legacyGUID", TestValue::UInt32(u32::MAX))
        .value("children", references(children))
}

fn tag_database(builder: DataForgeBuilder) -> DataForgeBuilder {
    let roots = [TAG_MOUNT, TAG_GUN, TAG_TURRET, TAG_ALPHA, TAG_BETA];
    [
        TestStruct::new("Tag")
            .field("tagName", TestType::String)
            .field("legacyGUID", TestType::UInt32)
            .array("children", TestType::ReferenceTo("Tag".into())),
        TestStruct::new("TagDatabase").array("tags", TestType::ReferenceTo("Tag".into())),
    ]
    .into_iter()
    .fold(builder, DataForgeBuilder::structure)
    .record(
        TestRecord::new(
            TAG_DATABASE,
            "TagDatabase",
            "TagDatabase.TagDatabase",
            TAG_PATH,
        )
        .value("tags", references(&roots)),
    )
    .record(tag(TAG_MOUNT, "Mount", &[]))
    .record(tag(TAG_GUN, "Gun", &[]))
    .record(tag(TAG_TURRET, "Turret", &[]))
    .record(tag(TAG_ALPHA, "Alpha", &[TAG_DUP_ALPHA]))
    .record(tag(TAG_BETA, "Beta", &[TAG_DUP_BETA]))
    .record(tag(TAG_DUP_ALPHA, "Dup", &[]))
    .record(tag(TAG_DUP_BETA, "Dup", &[]))
}

fn database() -> DataForgeBuilder {
    entities().iter().fold(
        tag_database(entity_schema(DataForgeBuilder::new())),
        |builder, entity| builder.record(entity.record()),
    )
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .entry(TestEntry::zstd(
            format!("{XML_DIR}ACME_Alpha.xml"),
            ALPHA_XML.as_bytes().to_vec(),
        ))
        .entry(TestEntry::zstd(
            format!("{XML_DIR}ACME_Broken.xml"),
            b"CryXmlB\0not really".to_vec(),
        ))
        .write_to(&path)?;
    Ok(path)
}
