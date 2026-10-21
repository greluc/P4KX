//! The commodity and resource-type Records of the crafting fixture.

use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestValue};

use super::{
    DOPPLE, GOLD, GROUP_ALLOY, GROUP_METAL, GROUP_MINERAL, RESOURCE_COPPER, RESOURCE_EMBER,
    RESOURCE_GOLD, RESOURCE_IRON,
};

fn commodity(
    guid: u128,
    class: &str,
    path: &str,
    names: (&str, &str),
    resource: Option<u128>,
) -> TestRecord {
    let (name, description) = names;
    let purchasable = TestInstance::new("SCItemPurchasableParams")
        .value("displayName", TestValue::text(name))
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

fn group(guid: u128, class: &str, name: &str, listed: (&[u128], &[u128])) -> TestRecord {
    let (resources, groups) = listed;
    TestRecord::new(
        guid,
        "ResourceTypeGroup",
        format!("ResourceTypeGroup.{class}"),
        "libs/foundry/records/resourcetypedatabase/resourcetypedatabase.xml",
    )
    .value("displayName", TestValue::text(name))
    .value(
        "groups",
        TestValue::Array(
            groups
                .iter()
                .map(|guid| TestValue::Reference(*guid))
                .collect(),
        ),
    )
    .value(
        "resources",
        TestValue::Array(
            resources
                .iter()
                .map(|guid| TestValue::Reference(*guid))
                .collect(),
        ),
    )
}

pub(super) fn prop(guid: u128, class: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/props/{class}.xml"),
    )
    .value("Components", TestValue::Array(Vec::new()))
}

pub(super) fn commodity_records(builder: DataForgeBuilder) -> DataForgeBuilder {
    builder
        .record(resource_type(RESOURCE_GOLD, "gold"))
        .record(resource_type(RESOURCE_IRON, "iron"))
        .record(resource_type(RESOURCE_EMBER, "ember"))
        .record(resource_type(RESOURCE_COPPER, "copper"))
        .record(group(
            GROUP_METAL,
            "Metal",
            "@items_commodities_type_metal",
            (&[RESOURCE_GOLD, RESOURCE_IRON], &[GROUP_ALLOY]),
        ))
        .record(group(
            GROUP_MINERAL,
            "Mineral",
            "@LOC_PLACEHOLDER",
            (&[RESOURCE_EMBER], &[]),
        ))
        .record(group(
            GROUP_ALLOY,
            "Alloy",
            "@items_commodities_type_alloy",
            (&[RESOURCE_COPPER], &[]),
        ))
        .record(commodity(
            GOLD,
            "gold",
            "metals/gold",
            ("@items_commodities_gold", "@LOC_UNINITIALIZED"),
            Some(RESOURCE_GOLD),
        ))
        .record(commodity(
            DOPPLE,
            "dopple",
            "vice/dopple",
            (
                "@items_commodities_dopple",
                "@items_commodities_dopple_desc",
            ),
            None,
        ))
        .record(commodity(
            0xa300_0000_0000_4000_8000_0000_0000_0003,
            "scrap_test",
            "test/scrap_test",
            ("@items_commodities_scrap_test", ""),
            None,
        ))
        .record(commodity(
            0xa500_0000_0000_4000_8000_0000_0000_0005,
            "ember",
            "vice/ember",
            ("@LOC_UNINITIALIZED", "@LOC_UNINITIALIZED"),
            Some(RESOURCE_EMBER),
        ))
        .record(prop(
            0xa400_0000_0000_4000_8000_0000_0000_0004,
            "Carryable_Gold",
        ))
}
