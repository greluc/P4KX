//! A synthetic Archive of ships that share vehicle definitions, Modifications and armour.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, GlobalIniBuilder,
    TestElement, TestEntry, TestInstance, TestRecord, TestStruct, TestType, TestValue,
};

pub(crate) const ALPHA_XML: &str = "scripts/entities/vehicles/implementations/xml/acas_alpha.xml";
pub(crate) const OTHER_XML: &str = "scripts/entities/vehicles/implementations/xml/acas_other.xml";
const BROKEN_XML: &str = "scripts/entities/vehicles/implementations/xml/acas_broken.xml";
const XML_DIR: &str = "Data\\Scripts\\Entities\\Vehicles\\Implementations\\Xml\\";

fn component(name: &str) -> TestStruct {
    TestStruct::new(name).parent("DataForgeComponentParams")
}

fn builder() -> DataForgeBuilder {
    [
        TestStruct::new("EntityClassDefinition").array(
            "Components",
            TestType::StrongPointer("DataForgeComponentParams".to_owned()),
        ),
        TestStruct::new("DataForgeComponentParams"),
        component("VehicleComponentParams")
            .field("vehicleDefinition", TestType::String)
            .field("modification", TestType::String)
            .field("vehicleName", TestType::Locale),
        component("SCItemVehicleArmorParams").field("damageMultiplier", TestType::Single),
        TestStruct::new("SCItemLocalization").field("Name", TestType::Locale),
        component("SAttachableComponentParams")
            .field("AttachDef", TestType::Class("SItemDefinition".to_owned())),
        TestStruct::new("SItemDefinition")
            .field("Type", TestType::Enum("EItemType".to_owned()))
            .field("Size", TestType::Int32)
            .field("Grade", TestType::Int32)
            .field("Tags", TestType::String)
            .field(
                "Localization",
                TestType::Class("SCItemLocalization".to_owned()),
            ),
        component("SEntityPhysicsControllerParams")
            .field("PhysType", TestType::Class("SEntityPhysicsType".to_owned())),
        TestStruct::new("SEntityPhysicsType").field("Mass", TestType::Single),
        component("SEntityComponentCarryableParams").field("carryInteraction", TestType::String),
    ]
    .into_iter()
    .fold(
        DataForgeBuilder::new()
            .version(8)
            .enumeration("EItemType", ["PowerPlant"]),
        DataForgeBuilder::structure,
    )
}

const fn guid(index: u128) -> u128 {
    0x2000_0000_0000_4000_8000_0000_0000_0000 + index
}

struct Item {
    class: &'static str,
    mass: Option<f32>,
    grade: i32,
    tags: &'static str,
}

const ITEMS: [Item; 6] = [
    Item {
        class: "POWR_Alpha",
        mass: Some(10.0),
        grade: 1,
        tags: "red",
    },
    Item {
        class: "POWR_Alpha_Blue",
        mass: Some(10.0),
        grade: 1,
        tags: "blue",
    },
    Item {
        class: "POWR_Heavy",
        mass: Some(11.0),
        grade: 1,
        tags: "red",
    },
    Item {
        class: "POWR_Grade2",
        mass: Some(10.0),
        grade: 2,
        tags: "red",
    },
    Item {
        class: "POWR_Bare_One",
        mass: None,
        grade: 1,
        tags: "red",
    },
    Item {
        class: "POWR_Bare_Two",
        mass: None,
        grade: 1,
        tags: "blue",
    },
];

fn item_record(index: usize, item: &Item) -> TestRecord {
    let attach = TestValue::Struct(
        TestInstance::new("SAttachableComponentParams").value(
            "AttachDef",
            TestValue::Struct(
                TestInstance::new("SItemDefinition")
                    .value("Type", TestValue::text("PowerPlant"))
                    .value("Size", TestValue::Int32(1))
                    .value("Grade", TestValue::Int32(item.grade))
                    .value("Tags", TestValue::text(item.tags))
                    .value(
                        "Localization",
                        TestValue::Struct(
                            TestInstance::new("SCItemLocalization")
                                .value("Name", TestValue::text(format!("@item_Name{index}"))),
                        ),
                    ),
            ),
        ),
    );
    let mut components = vec![
        attach,
        TestValue::Struct(
            TestInstance::new("SEntityComponentCarryableParams")
                .value("carryInteraction", TestValue::text(item.tags)),
        ),
    ];
    if let Some(mass) = item.mass {
        components.push(TestValue::Struct(
            TestInstance::new("SEntityPhysicsControllerParams").value(
                "PhysType",
                TestValue::Struct(
                    TestInstance::new("SEntityPhysicsType").value("Mass", TestValue::Single(mass)),
                ),
            ),
        ));
    }
    TestRecord::new(
        guid(0x100 + index as u128),
        "EntityClassDefinition",
        format!("EntityClassDefinition.{}", item.class),
        format!(
            "libs/foundry/records/entities/scitem/ships/powerplant/{}.xml",
            item.class
        ),
    )
    .value("Components", TestValue::Array(components))
}

struct Ship {
    class: &'static str,
    definition: &'static str,
    modification: &'static str,
    armour: Option<f32>,
}

const fn ship(
    class: &'static str,
    definition: &'static str,
    modification: &'static str,
    armour: Option<f32>,
) -> Ship {
    Ship {
        class,
        definition,
        modification,
        armour,
    }
}

const SHIPS: [Ship; 11] = [
    ship("ACAS_Alpha", ALPHA_XML, "", None),
    ship("ACAS_Alpha_Edition", ALPHA_XML, "", None),
    ship("ACAS_Alpha_Zulu", ALPHA_XML, "", None),
    ship("ACAS_Beta", ALPHA_XML, "Beta", None),
    ship("ACAS_Beta_Edition", ALPHA_XML, "Beta", None),
    ship("ACAS_Alpha_Plated", ALPHA_XML, "", Some(0.5)),
    ship("ACAS_Other", OTHER_XML, "", None),
    ship("ACAS_Lonely", "", "", None),
    ship("ACAS_Broken", BROKEN_XML, "", None),
    ship(
        "ACAS_Missing",
        "scripts/entities/vehicles/implementations/xml/none.xml",
        "",
        None,
    ),
    ship("ACAS_Solo", OTHER_XML, "Beta", None),
];

fn record(index: usize, ship: &Ship) -> TestRecord {
    let mut components = vec![TestValue::Struct(
        TestInstance::new("VehicleComponentParams")
            .value("vehicleDefinition", TestValue::text(ship.definition))
            .value("modification", TestValue::text(ship.modification))
            .value(
                "vehicleName",
                TestValue::text(format!("@vehicle_Name{index}")),
            ),
    )];
    if let Some(multiplier) = ship.armour {
        components.push(TestValue::Struct(
            TestInstance::new("SCItemVehicleArmorParams")
                .value("damageMultiplier", TestValue::Single(multiplier)),
        ));
    }
    TestRecord::new(
        guid(index as u128 + 1),
        "EntityClassDefinition",
        format!("EntityClassDefinition.{}", ship.class),
        format!(
            "libs/foundry/records/entities/spaceships/{}.xml",
            ship.class
        ),
    )
    .value("Components", TestValue::Array(components))
}

fn port(tags: &str) -> TestElement {
    TestElement::new("Part")
        .attribute("class", "ItemPort")
        .child(
            TestElement::new("ItemPort")
                .attribute("id", "modPaint")
                .attribute("portTags", tags)
                .attribute("requiredTags", tags)
                .child(
                    TestElement::new("Types")
                        .child(TestElement::new("Type").attribute("type", "Paints")),
                ),
        )
}

fn vehicle_xml(tags: &str) -> Vec<u8> {
    TestElement::new("Vehicle")
        .child(TestElement::new("Parts").child(port(tags)))
        .child(
            TestElement::new("Modifications").child(
                TestElement::new("Modification")
                    .attribute("name", "Beta")
                    .child(
                        TestElement::new("Elems").child(
                            TestElement::new("Elem")
                                .attribute("idRef", "modPaint")
                                .attribute("name", "requiredTags")
                                .attribute("value", "Paint_Beta"),
                        ),
                    ),
            ),
        )
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let database = SHIPS
        .iter()
        .enumerate()
        .fold(builder(), |database, (index, ship)| {
            database.record(record(index, ship))
        });
    let database = ITEMS
        .iter()
        .enumerate()
        .fold(database, |database, (index, item)| {
            database.record(item_record(index, item))
        });
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database.build()).encrypted())
        .entry(TestEntry::zstd(
            "Data\\Localization\\english\\global.ini",
            GlobalIniBuilder::new().build(),
        ))
        .entry(TestEntry::zstd(
            format!("{XML_DIR}ACAS_Alpha.xml"),
            vehicle_xml("Paint_Alpha"),
        ))
        .entry(TestEntry::zstd(
            format!("{XML_DIR}ACAS_Other.xml"),
            vehicle_xml("Paint_Alpha"),
        ))
        .entry(TestEntry::zstd(
            format!("{XML_DIR}ACAS_Broken.xml"),
            b"CryXmlB\0not really".to_vec(),
        ))
        .write_to(&path)?;
    Ok(path)
}
