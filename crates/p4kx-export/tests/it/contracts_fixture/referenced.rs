use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestValue};

use super::lookups::{mission_type, plain_record};
use super::records::{organization, plain, property, reference, string_hash};
use super::{
    BOUNTY, DATABASE, DELIVERY, ESCORT, ORGANIZATION, PLAIN_TEMPLATE, SOLO_ORGANIZATION,
    STRAY_TEMPLATE, TAG_OTHER, TAG_TITLE, TEMPLATE,
};

const TAG_PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

fn template(
    guid: u128,
    name: &str,
    title: &str,
    kind: u128,
    properties: Vec<TestValue>,
) -> TestRecord {
    template_at(
        guid,
        name,
        &format!("contracts/contracttemplates/{name}"),
        (title, kind),
        properties,
    )
}

fn template_at(
    guid: u128,
    name: &str,
    path: &str,
    (title, kind): (&str, u128),
    properties: Vec<TestValue>,
) -> TestRecord {
    TestRecord::new(
        guid,
        "ContractTemplate",
        format!("ContractTemplate.{name}"),
        format!("libs/foundry/records/{path}.xml"),
    )
    .value(
        "contractDisplayInfo",
        TestValue::Struct(
            TestInstance::new("ContractDisplayInfo")
                .value("type", reference(kind))
                .value(
                    "displayString",
                    TestValue::Array(vec![TestValue::Struct(
                        TestInstance::new("LocID").value("value", TestValue::text(title)),
                    )]),
                ),
        ),
    )
    .value("contractProperties", TestValue::Array(properties))
}

fn tag(guid: u128, name: &str) -> TestRecord {
    TestRecord::new(guid, "Tag", format!("Tag.{name}"), TAG_PATH)
        .value("tagName", TestValue::text(name))
        .value("legacyGUID", TestValue::UInt32(u32::MAX))
}

fn variant(tag: u128, key: &str) -> TestValue {
    TestValue::Struct(
        TestInstance::new("MissionStringVariant")
            .value("tag", reference(tag))
            .value("string", TestValue::text(key)),
    )
}

fn organisation(guid: u128, name: &str, variants: Vec<TestValue>) -> TestRecord {
    plain_record(
        guid,
        "MissionOrganization",
        &format!("missiondata/pu_organizations/{name}"),
    )
    .value(
        "stringVariants",
        TestValue::Struct(
            TestInstance::new("MissionStringVariants")
                .value("variants", TestValue::Array(variants)),
        ),
    )
}

fn templates() -> Vec<TestRecord> {
    let delivery_properties = vec![
        property("Cargo", string_hash("@cargo_gold")),
        property("Location", plain("MissionPropertyValue_Location")),
        property("Contractor", organization(ORGANIZATION)),
        property("Target", plain("MissionPropertyValue_AIName")),
        property("Ship", plain("MissionPropertyValue_ShipSpawnDescriptions")),
        property("Flag", plain("MissionPropertyValue_Boolean")),
    ];
    vec![
        template(
            TEMPLATE,
            "delivery",
            "@title_template",
            DELIVERY,
            delivery_properties,
        ),
        template(
            PLAIN_TEMPLATE,
            "plain",
            "@title_plain",
            DELIVERY,
            Vec::new(),
        ),
        template_at(
            STRAY_TEMPLATE,
            "stray",
            "stray_template",
            ("@title_plain", DELIVERY),
            Vec::new(),
        ),
    ]
}

fn organisations() -> Vec<TestRecord> {
    vec![
        TestRecord::new(DATABASE, "TagDatabase", "TagDatabase.TagDatabase", TAG_PATH).value(
            "tags",
            TestValue::Array(vec![reference(TAG_TITLE), reference(TAG_OTHER)]),
        ),
        tag(TAG_TITLE, "Title"),
        tag(TAG_OTHER, "Other"),
        organisation(
            SOLO_ORGANIZATION,
            "solo",
            vec![variant(TAG_OTHER, "@org_solo")],
        ),
        organisation(
            ORGANIZATION,
            "acme",
            vec![
                variant(TAG_OTHER, "@org_acme_other"),
                variant(TAG_TITLE, "@org_acme_title"),
            ],
        ),
    ]
}

pub(super) fn referenced(builder: DataForgeBuilder) -> DataForgeBuilder {
    let types = [
        mission_type(DELIVERY, "delivery", "@type_delivery"),
        mission_type(BOUNTY, "bounty", "@type_bounty"),
        mission_type(ESCORT, "escort", "@type_escort"),
    ];
    templates()
        .into_iter()
        .chain(types)
        .chain(organisations())
        .fold(builder, DataForgeBuilder::record)
}
