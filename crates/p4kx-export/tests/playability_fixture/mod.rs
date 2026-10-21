//! A synthetic Archive whose ships and items cover every playability value and `null`.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestInstance, TestRecord,
    TestStruct, TestType, TestValue,
};

const CREWED_MANIFEST: u128 = 0x1100_0000_0000_4000_8000_0000_0000_0001;
const EMPTY_MANIFEST: u128 = 0x1100_0000_0000_4000_8000_0000_0000_0002;
const UNMANNED_TAG: u128 = 0x1200_0000_0000_4000_8000_0000_0000_0001;
const OTHER_TAG: u128 = 0x1200_0000_0000_4000_8000_0000_0000_0002;

/// How one synthetic ship is set up.
#[derive(Clone, Copy)]
pub(crate) struct ShipSpec {
    pub(crate) class: &'static str,
    pub(crate) folder: &'static str,
    pub(crate) inclusion: Option<&'static str>,
    pub(crate) manifest: Option<u128>,
    pub(crate) explosion: f32,
    pub(crate) hull_explodes: bool,
    pub(crate) tags: &'static [u128],
}

const PLAIN: ShipSpec = ShipSpec {
    class: "",
    folder: "spaceships",
    inclusion: Some("ReadyToInclude"),
    manifest: None,
    explosion: 0.0,
    hull_explodes: false,
    tags: &[OTHER_TAG],
};

pub(crate) const SHIPS: [ShipSpec; 11] = [
    ShipSpec {
        class: "Ship_Player",
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Crewed",
        manifest: Some(CREWED_MANIFEST),
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Explosive",
        explosion: 1.0,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Hull_Not_Shipped",
        inclusion: Some("DoNotInclude"),
        hull_explodes: true,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Salvage",
        tags: &[OTHER_TAG, UNMANNED_TAG],
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Not_Shipped",
        inclusion: Some("DoNotInclude"),
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Ambiguous",
        hull_explodes: true,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_No_Inclusion",
        inclusion: None,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Empty_Manifest",
        manifest: Some(EMPTY_MANIFEST),
        ..PLAIN
    },
    ShipSpec {
        class: "Rover_Player",
        folder: "groundvehicles",
        explosion: 1.0,
        hull_explodes: true,
        ..PLAIN
    },
    ShipSpec {
        class: "Rover_Salvage",
        folder: "groundvehicles",
        tags: &[UNMANNED_TAG],
        ..PLAIN
    },
];

/// How one synthetic item is set up.
#[derive(Clone, Copy)]
pub(crate) struct ItemSpec {
    pub(crate) class: &'static str,
    pub(crate) path: &'static str,
    pub(crate) name: Option<&'static str>,
    pub(crate) shop: Option<&'static str>,
}

pub(crate) const ITEMS: [ItemSpec; 7] = [
    ItemSpec {
        class: "Item_Shop",
        path: "scitem/ships/powerplant/item_shop",
        name: Some("@item_NameShop"),
        shop: Some("@item_NameShop"),
    },
    ItemSpec {
        class: "Item_Shop_Without_Name",
        path: "scitem/ships/powerplant/item_shop_without_name",
        name: None,
        shop: Some("@item_NameShop"),
    },
    ItemSpec {
        class: "Item_Mission",
        path: "scitem/mission_entities/item_mission",
        name: None,
        shop: None,
    },
    ItemSpec {
        class: "Item_Mission_Stash",
        path: "scitem/missionstorage/item_mission_stash",
        name: Some("@item_NameStash"),
        shop: None,
    },
    ItemSpec {
        class: "Item_Nameless",
        path: "scitem/doors/item_nameless",
        name: Some("@LOC_PLACEHOLDER"),
        shop: Some("@LOC_PLACEHOLDER"),
    },
    ItemSpec {
        class: "Item_Named",
        path: "scitem/carryables/item_named",
        name: Some("@item_NameNamed"),
        shop: None,
    },
    ItemSpec {
        class: "Item_Old",
        path: "legacyitems/item_old",
        name: None,
        shop: None,
    },
];

fn guid(kind: u128, index: usize) -> u128 {
    (kind << 120) | 0x4000_8000_0000_0000_0000 | (u128::try_from(index).unwrap_or(0) + 1)
}

fn class(name: &str) -> TestType {
    TestType::Class(name.to_owned())
}

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn component(name: &str) -> TestStruct {
    TestStruct::new(name).parent("DataForgeComponentParams")
}

fn structures(builder: DataForgeBuilder) -> DataForgeBuilder {
    [
        TestStruct::new("Tag").field("tagName", TestType::String),
        TestStruct::new("CrewEntry").field("name", TestType::String),
        TestStruct::new("CrewManifest").array("crew", class("CrewEntry")),
        TestStruct::new("SCItemLocalization").field("Name", TestType::Locale),
        TestStruct::new("SItemDefinition").field("Localization", class("SCItemLocalization")),
        TestStruct::new("SAIEngineeringOptions")
            .field("powerPlantsCanExplodeOnDeath", TestType::Bool),
        TestStruct::new("StaticEntityClassData"),
        TestStruct::new("EAEntityDataParams")
            .parent("StaticEntityClassData")
            .field("inclusionMode", TestType::Enum("EInclusionMode".to_owned())),
        TestStruct::new("EntityClassDefinition")
            .field("Category", TestType::String)
            .array("Components", pointer("DataForgeComponentParams"))
            .array("tags", TestType::Reference)
            .array("StaticEntityClassData", pointer("StaticEntityClassData")),
        TestStruct::new("DataForgeComponentParams"),
        component("SAttachableComponentParams").field("AttachDef", class("SItemDefinition")),
        component("SCItemPurchasableParams").field("displayName", TestType::Locale),
        component("VehicleComponentParams")
            .field("criticalPartExplosionChance", TestType::Single)
            .field("aiEngineeringOptions", class("SAIEngineeringOptions")),
        component("SEntityComponentCrewManifestParams").field("crewManifest", TestType::Reference),
    ]
    .into_iter()
    .fold(builder, DataForgeBuilder::structure)
}

fn entity(
    guid: u128,
    class: &str,
    path: &str,
    components: Vec<TestValue>,
    tags: &[u128],
    inclusion: Option<&str>,
) -> TestRecord {
    let data = inclusion.map_or_else(Vec::new, |mode| {
        vec![TestValue::Struct(
            TestInstance::new("EAEntityDataParams").value("inclusionMode", TestValue::text(mode)),
        )]
    });
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/{path}.xml"),
    )
    .value("Category", TestValue::text("Default"))
    .value("Components", TestValue::Array(components))
    .value(
        "tags",
        TestValue::Array(tags.iter().copied().map(TestValue::Reference).collect()),
    )
    .value("StaticEntityClassData", TestValue::Array(data))
}

fn ship(index: usize, spec: &ShipSpec) -> TestRecord {
    let mut components = vec![TestValue::Struct(
        TestInstance::new("VehicleComponentParams")
            .value(
                "criticalPartExplosionChance",
                TestValue::Single(spec.explosion),
            )
            .value(
                "aiEngineeringOptions",
                TestValue::Struct(TestInstance::new("SAIEngineeringOptions").value(
                    "powerPlantsCanExplodeOnDeath",
                    TestValue::Bool(spec.hull_explodes),
                )),
            ),
    )];
    if let Some(manifest) = spec.manifest {
        components.push(TestValue::Struct(
            TestInstance::new("SEntityComponentCrewManifestParams")
                .value("crewManifest", TestValue::Reference(manifest)),
        ));
    }
    entity(
        guid(0x90, index),
        spec.class,
        &format!("{}/{}", spec.folder, spec.class.to_ascii_lowercase()),
        components,
        spec.tags,
        spec.inclusion,
    )
}

fn item(index: usize, spec: &ItemSpec) -> TestRecord {
    let mut components = Vec::new();
    if let Some(name) = spec.name {
        components.push(TestValue::Struct(
            TestInstance::new("SAttachableComponentParams").value(
                "AttachDef",
                TestValue::Struct(
                    TestInstance::new("SItemDefinition").value(
                        "Localization",
                        TestValue::Struct(
                            TestInstance::new("SCItemLocalization")
                                .value("Name", TestValue::text(name)),
                        ),
                    ),
                ),
            ),
        ));
    }
    if let Some(shop) = spec.shop {
        components.push(TestValue::Struct(
            TestInstance::new("SCItemPurchasableParams")
                .value("displayName", TestValue::text(shop)),
        ));
    }
    entity(
        guid(0x30, index),
        spec.class,
        spec.path,
        components,
        &[],
        None,
    )
}

fn manifest(guid: u128, name: &str, crew: usize) -> TestRecord {
    TestRecord::new(
        guid,
        "CrewManifest",
        format!("CrewManifest.{name}"),
        format!("libs/foundry/records/crewmanifest/{name}.xml"),
    )
    .value(
        "crew",
        TestValue::Array(
            (0..crew)
                .map(|_| {
                    TestValue::Struct(
                        TestInstance::new("CrewEntry").value("name", TestValue::text("pilot")),
                    )
                })
                .collect(),
        ),
    )
}

fn tag(guid: u128, name: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "Tag",
        format!("Tag.{name}"),
        "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase",
    )
    .value("tagName", TestValue::text(name))
}

fn database() -> DataForgeBuilder {
    let builder = structures(
        DataForgeBuilder::new()
            .version(8)
            .enumeration("EInclusionMode", ["ReadyToInclude", "DoNotInclude"]),
    )
    .record(manifest(CREWED_MANIFEST, "crewed", 2))
    .record(manifest(EMPTY_MANIFEST, "crewless", 0))
    .record(tag(UNMANNED_TAG, "Unmanned"))
    .record(tag(OTHER_TAG, "Ship"));
    let builder = SHIPS
        .iter()
        .enumerate()
        .fold(builder, |builder, (index, spec)| {
            builder.record(ship(index, spec))
        });
    ITEMS
        .iter()
        .enumerate()
        .fold(builder, |builder, (index, spec)| {
            builder.record(item(index, spec))
        })
}

fn english() -> TestEntry {
    TestEntry::zstd(
        "Data\\Localization\\english\\global.ini",
        GlobalIniBuilder::new()
            .text("item_NameShop", "Shop item")
            .text("item_NameStash", "Stash")
            .text("item_NameNamed", "Named item")
            .build(),
    )
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .entry(english())
        .write_to(&path)?;
    Ok(path)
}
