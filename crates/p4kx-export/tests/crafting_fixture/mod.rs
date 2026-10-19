//! A synthetic Archive with commodities and crafting blueprints for the Basetool catalog tests.

use std::error::Error;
use std::path::{Path, PathBuf};

mod structures;

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestInstance, TestRecord,
    TestValue,
};

pub(crate) const GOLD: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const DOPPLE: u128 = 0xa200_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const RESOURCE_GOLD: u128 = 0xb100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const RESOURCE_IRON: u128 = 0xb200_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const TAG_GOLD: u128 = 0xc100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const CATEGORY: u128 = 0xd100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const PRODUCT: u128 = 0xe100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const ORE: u128 = 0xe200_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const MISSING: u128 = 0xe300_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const MAGAZINE: u128 = 0x1100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const CORE: u128 = 0x1200_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const DANGLING: u128 = 0x1300_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const TEMPLATE: u128 = 0x1400_0000_0000_4000_8000_0000_0000_0004;

pub(crate) fn resource(guid: u128, scu: f32, quality: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CraftingCost_Resource")
            .value("minQuality", TestValue::Int32(quality))
            .value("resource", TestValue::Reference(guid))
            .value(
                "quantity",
                TestValue::Struct(
                    TestInstance::new("SStandardCargoUnit")
                        .value("standardCargoUnits", TestValue::Single(scu)),
                ),
            ),
    )
}

pub(crate) fn item(guid: u128, units: i32, quality: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CraftingCost_Item")
            .value("quantity", TestValue::Int32(units))
            .value("minQuality", TestValue::Int32(quality))
            .value("entityClass", TestValue::Reference(guid)),
    )
}

pub(crate) fn select(debug: &str, key: &str, options: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CraftingCost_Select")
            .value(
                "nameInfo",
                TestValue::Struct(
                    TestInstance::new("CraftingNameInfo")
                        .value("debugName", TestValue::text(debug))
                        .value("displayName", TestValue::text(key)),
                ),
            )
            .value("options", TestValue::Array(options)),
    )
}

fn time(days: i32, hours: i32, minutes: i32, seconds: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("TimeValue_Partitioned")
            .value("days", TestValue::Int32(days))
            .value("hours", TestValue::Int32(hours))
            .value("minutes", TestValue::Int32(minutes))
            .value("seconds", TestValue::Single(seconds)),
    )
}

/// What one synthetic blueprint holds.
pub(crate) struct BlueprintSpec {
    pub(crate) guid: u128,
    pub(crate) class: &'static str,
    pub(crate) path: &'static str,
    pub(crate) produces: u128,
    pub(crate) mandatory: TestValue,
}

fn blueprint(spec: &BlueprintSpec) -> TestRecord {
    let costs = TestInstance::new("CraftingRecipeCosts")
        .value("craftTime", time(1, 2, 3, 3.6))
        .value("mandatoryCost", spec.mandatory.clone())
        .value("optionalCosts", TestValue::Array(Vec::new()));
    let tier = TestInstance::new("CraftingBlueprintTier").value(
        "recipe",
        TestValue::Struct(
            TestInstance::new("CraftingRecipe").value("costs", TestValue::Struct(costs)),
        ),
    );
    TestRecord::new(
        spec.guid,
        "CraftingBlueprintRecord",
        format!("CraftingBlueprintRecord.{}", spec.class),
        format!("libs/foundry/records/crafting/blueprints/{}.xml", spec.path),
    )
    .value(
        "blueprint",
        TestValue::Struct(
            TestInstance::new("CraftingBlueprint")
                .value("blueprintName", TestValue::text("@LOC_PLACEHOLDER"))
                .value("category", TestValue::Reference(CATEGORY))
                .value(
                    "processSpecificData",
                    TestValue::Struct(
                        TestInstance::new("CraftingProcess_Creation")
                            .value("entityClass", TestValue::Reference(spec.produces)),
                    ),
                )
                .value("tiers", TestValue::Array(vec![TestValue::Struct(tier)])),
        ),
    )
}

fn commodity(
    guid: u128,
    class: &str,
    path: &str,
    description: &str,
    resource: Option<u128>,
) -> TestRecord {
    let purchasable = TestInstance::new("SCItemPurchasableParams")
        .value(
            "displayName",
            TestValue::text(format!("@items_commodities_{class}")),
        )
        .value(
            "displayType",
            TestValue::text("@items_commodities_type_metal"),
        );
    let container = TestInstance::new("ResourceContainer").value(
        "defaultComposition",
        TestValue::Array(
            resource
                .map(|guid| {
                    TestValue::Struct(
                        TestInstance::new("ResourceContainerDefaultCompositionEntry")
                            .value("entry", TestValue::Reference(guid)),
                    )
                })
                .into_iter()
                .collect(),
        ),
    );
    let display = TestInstance::new("EntityUIDisplayParams")
        .value("displayDescription", TestValue::text(description));
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/commodities/{path}.xml"),
    )
    .value(
        "tags",
        TestValue::Array(vec![TestValue::Reference(TAG_GOLD)]),
    )
    .value(
        "StaticEntityClassData",
        TestValue::Array(vec![TestValue::Struct(display)]),
    )
    .value(
        "Components",
        TestValue::Array(vec![
            TestValue::Struct(purchasable),
            TestValue::Struct(container),
        ]),
    )
}

fn resource_type(guid: u128, class: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "ResourceType",
        format!("ResourceType.{class}"),
        "libs/foundry/records/resourcetypedatabase/resourcetypedatabase.xml",
    )
    .value(
        "displayName",
        TestValue::text(format!("@items_commodities_{class}")),
    )
    .value(
        "description",
        TestValue::text(format!("@items_commodities_{class}_desc")),
    )
}

fn prop(guid: u128, class: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/props/{class}.xml"),
    )
    .value("Components", TestValue::Array(Vec::new()))
}

/// The blueprints of the fixture in the order of their GUIDs.
pub(crate) fn blueprints() -> Vec<BlueprintSpec> {
    let magazine = BlueprintSpec {
        guid: MAGAZINE,
        class: "BP_CRAFT_behr_lmg_01_mag",
        path: "crafting/fpsgear/ammo/bp_craft_behr_lmg_01_mag",
        produces: PRODUCT,
        mandatory: select(
            "ASPECTS",
            "@LOC_PLACEHOLDER",
            vec![
                select(
                    "MAGAZINE",
                    "@crafting_ui_slotname_magazine",
                    vec![resource(RESOURCE_GOLD, 0.5, 0)],
                ),
                select(
                    "CORE",
                    "@LOC_PLACEHOLDER",
                    vec![item(ORE, 5, 500), resource(RESOURCE_IRON, 0.25, 300)],
                ),
                resource(RESOURCE_IRON, 2.0, 100),
            ],
        ),
    };
    let core = BlueprintSpec {
        guid: CORE,
        class: "BP_CRAFT_core",
        path: "crafting/vehiclegear/powerplant/bp_craft_core",
        produces: ORE,
        mandatory: resource(RESOURCE_GOLD, 1.0, 0),
    };
    let dangling = BlueprintSpec {
        guid: DANGLING,
        class: "BP_CRAFT_dangling",
        path: "crafting/vehiclegear/cooler/bp_craft_dangling",
        produces: MISSING,
        mandatory: item(ORE, 1, 0),
    };
    let template = BlueprintSpec {
        guid: TEMPLATE,
        class: "BP_CRAFT_weapon",
        path: "crafting/fpsgear/weapons/$templates/bp_craft_weapon",
        produces: PRODUCT,
        mandatory: resource(RESOURCE_IRON, 1.0, 0),
    };
    let leftover = BlueprintSpec {
        guid: 0x1500_0000_0000_4000_8000_0000_0000_0005,
        class: "BP_CRAFT_dev_test",
        path: "crafting/fpsgear/bp_craft_dev_test",
        produces: PRODUCT,
        mandatory: resource(RESOURCE_IRON, 1.0, 0),
    };
    vec![magazine, core, dangling, template, leftover]
}

/// Returns the DataForge of the fixture.
pub(crate) fn database() -> DataForgeBuilder {
    let builder = commodity_records(structures::structures(DataForgeBuilder::new().version(8)));
    blueprints()
        .iter()
        .fold(blueprint_records(builder), |builder, spec| {
            builder.record(blueprint(spec))
        })
}

fn blueprint_records(builder: DataForgeBuilder) -> DataForgeBuilder {
    builder
        .record(TestRecord::new(
            CATEGORY,
            "BlueprintCategoryRecord",
            "BlueprintCategoryRecord.Ammo",
            "libs/foundry/records/crafting/blueprintcategories/blueprintcategorydatabase.xml",
        ))
        .record(prop(PRODUCT, "Magazine"))
        .record(prop(ORE, "Ore"))
        .record(
            TestRecord::new(
                0x1600_0000_0000_4000_8000_0000_0000_0006,
                "CraftingBlueprintRecord",
                "CraftingBlueprintRecord.GlobalGenericDismantle",
                "libs/foundry/records/crafting/blueprints/dismantle/globalgenericdismantle.xml",
            )
            .value(
                "blueprint",
                TestValue::Struct(
                    TestInstance::new("GenericCraftingBlueprint").value(
                        "processSpecificData",
                        TestValue::Struct(
                            TestInstance::new("CraftingProcess_Creation")
                                .value("entityClass", TestValue::Reference(PRODUCT)),
                        ),
                    ),
                ),
            ),
        )
}

fn commodity_records(builder: DataForgeBuilder) -> DataForgeBuilder {
    builder
        .record(resource_type(RESOURCE_GOLD, "gold"))
        .record(resource_type(RESOURCE_IRON, "iron"))
        .record(
            TestRecord::new(
                TAG_GOLD,
                "Tag",
                "Tag.Gold",
                "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml",
            )
            .value("tagName", TestValue::text("Gold")),
        )
        .record(commodity(
            GOLD,
            "gold",
            "metals/gold",
            "@LOC_UNINITIALIZED",
            Some(RESOURCE_GOLD),
        ))
        .record(commodity(
            DOPPLE,
            "dopple",
            "vice/dopple",
            "@items_commodities_dopple_desc",
            None,
        ))
        .record(commodity(
            0xa300_0000_0000_4000_8000_0000_0000_0003,
            "scrap_test",
            "test/scrap_test",
            "",
            None,
        ))
        .record(prop(
            0xa400_0000_0000_4000_8000_0000_0000_0004,
            "Carryable_Gold",
        ))
}

/// Returns the English `global.ini` of the fixture.
pub(crate) fn english() -> TestEntry {
    TestEntry::zstd(
        "Data\\Localization\\english\\global.ini",
        GlobalIniBuilder::new()
            .text("items_commodities_gold", "Gold")
            .text("items_commodities_gold_desc", "A precious metal.")
            .text("items_commodities_dopple", "Dopple")
            .text("items_commodities_dopple_desc", "A vice.")
            .text("items_commodities_type_metal", "Metal")
            .build(),
    )
}

/// Returns the German `global.ini` of the fixture.
pub(crate) fn german() -> TestEntry {
    TestEntry::zstd(
        "Data\\Localization\\german_(germany)\\global.ini",
        GlobalIniBuilder::new()
            .text("items_commodities_gold", "Gold")
            .text("items_commodities_gold_desc", "Ein Edelmetall.")
            .text("items_commodities_type_metal", "Metall")
            .build(),
    )
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .entry(english())
        .entry(german())
        .write_to(&path)?;
    Ok(path)
}
