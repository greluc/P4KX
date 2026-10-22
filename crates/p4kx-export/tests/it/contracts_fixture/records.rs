use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestValue};

use super::lookups::{legacy, mission_type, plain_record};
use super::{
    AFFINITY, BOUNTY, BROKER, CONTRACT_A, CONTRACT_B, CONTRACT_C, CONTRACT_D, CONTRACT_DEV,
    CONTRACT_E, DATABASE, DELIVERY, ESCORT, FACTION, FACTION_SCOPE, GENERATOR, LOC_A, LOC_B, LOSS,
    OBJ_PLANET, ORGANIZATION, PLAIN_TEMPLATE, RANK_FIFTY, RANK_TWO_HUNDRED, SUB_ONE, SUB_TWO,
    TAG_OTHER, TAG_TITLE, TEMPLATE,
};

const TAG_PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

const fn reference(guid: u128) -> TestValue {
    TestValue::Reference(guid)
}

fn param(name: &str, key: &str) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractStringParam")
            .value("param", TestValue::text(name))
            .value("value", TestValue::text(key)),
    )
}

fn property(token: &str, value: TestValue) -> TestValue {
    TestValue::Struct(
        TestInstance::new("MissionProperty")
            .value(
                "missionVariableName",
                TestValue::text(format!("Var{token}")),
            )
            .value("extendedTextToken", TestValue::text(token))
            .value("value", value),
    )
}

fn string_hash(key: &str) -> TestValue {
    TestValue::Struct(TestInstance::new("MissionPropertyValue_StringHash").value(
        "options",
        TestValue::Array(vec![TestValue::Struct(
                TestInstance::new("MissionPropertyValueOption_StringHash")
                    .value("textId", TestValue::text(key))
                    .value("weighting", TestValue::Int32(1)),
            )]),
    ))
}

fn organization(guid: u128) -> TestValue {
    TestValue::Struct(
        TestInstance::new("MissionPropertyValue_Organization").value(
            "matchConditions",
            TestValue::Array(vec![TestValue::Struct(
                TestInstance::new("DataSetMatchCondition_SpecificOrganizationsDef")
                    .value("organizations", TestValue::Array(vec![reference(guid)])),
            )]),
        ),
    )
}

fn plain(kind: &str) -> TestValue {
    TestValue::Struct(TestInstance::new(kind))
}

fn overrides(
    title: Option<&str>,
    mission_type: Option<u128>,
    properties: Vec<TestValue>,
) -> TestValue {
    let mut params = TestInstance::new("ContractParamOverrides");
    if let Some(guid) = mission_type {
        params = params.value("missionTypeOverride", reference(guid));
    }
    TestValue::Struct(
        params
            .value(
                "stringParamOverrides",
                TestValue::Array(title.map(|key| param("Title", key)).into_iter().collect()),
            )
            .value("propertyOverrides", TestValue::Array(properties)),
    )
}

fn amount(scope: u128, reward: u128) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ReputationAmount")
            .value("factionReputation", reference(FACTION))
            .value("reputationScope", reference(scope))
            .value("reward", reference(reward)),
    )
}

fn results(results: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractResultBlock")
            .value("contractResults", TestValue::Array(results)),
    )
}

fn contract(id: u128, name: &str, template: u128, params: TestValue) -> TestInstance {
    TestInstance::new("CareerContract")
        .value("id", TestValue::Guid(id))
        .value("debugName", TestValue::text(name))
        .value("template", reference(template))
        .value("paramOverrides", params)
}

fn sub(id: u128, params: TestValue, prerequisites: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("SubContract")
            .value("id", TestValue::Guid(id))
            .value("additionalPrerequisites", TestValue::Array(prerequisites))
            .value("paramOverrides", params),
    )
}

fn locality(guid: u128) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractPrerequisite_Locality")
            .value("localityAvailable", reference(guid)),
    )
}

fn location(guid: u128) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractPrerequisite_Location")
            .value("locationAvailable", reference(guid)),
    )
}

fn handler(
    name: &str,
    params: TestValue,
    contracts: Vec<TestValue>,
    intro: Vec<TestValue>,
    faction: u128,
) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractGeneratorHandler_Career")
            .value("debugName", TestValue::text(name))
            .value("factionReputation", reference(faction))
            .value("contractParams", params)
            .value("contracts", TestValue::Array(contracts))
            .value("introContracts", TestValue::Array(intro)),
    )
}

fn delivery_contract() -> TestInstance {
    contract(
        CONTRACT_A,
        "Deliver_A",
        TEMPLATE,
        overrides(
            Some("@title_a"),
            None,
            vec![property("Cargo", string_hash("@cargo_scrap"))],
        ),
    )
    .value(
        "subContracts",
        TestValue::Array(vec![
            sub(
                SUB_ONE,
                overrides(Some("@title_sub"), Some(ESCORT), Vec::new()),
                vec![location(OBJ_PLANET)],
            ),
            sub(SUB_TWO, overrides(None, None, Vec::new()), Vec::new()),
        ]),
    )
    .value(
        "additionalPrerequisites",
        TestValue::Array(vec![locality(LOC_A)]),
    )
    .value(
        "contractResults",
        results(vec![
            legacy(
                [true, false, false, false, false],
                vec![
                    amount(FACTION_SCOPE, RANK_TWO_HUNDRED),
                    amount(AFFINITY, RANK_FIFTY),
                ],
            ),
            legacy(
                [false, false, true, false, false],
                vec![amount(FACTION_SCOPE, LOSS)],
            ),
            TestValue::Struct(TestInstance::new("ContractResult_CompletionTags")),
        ]),
    )
}

fn contractor_params(key: &str) -> TestValue {
    TestValue::Struct(TestInstance::new("ContractParamOverrides").value(
        "stringParamOverrides",
        TestValue::Array(vec![param("Contractor", key)]),
    ))
}

fn bounty_handler() -> TestValue {
    let bounty = contract(
        CONTRACT_C,
        "Bounty_C",
        PLAIN_TEMPLATE,
        overrides(None, None, Vec::new()),
    );
    let leftover = contract(
        CONTRACT_DEV,
        "Test_Contract",
        TEMPLATE,
        overrides(Some("@title_a"), None, Vec::new()),
    );
    let without_id = TestInstance::new("CareerContract")
        .value("debugName", TestValue::text("Deliver_NoId"))
        .value("template", reference(TEMPLATE));
    handler(
        "Bounty_Career",
        overrides(Some("@title_handler"), Some(BOUNTY), Vec::new()),
        vec![TestValue::Struct(bounty)],
        vec![TestValue::Struct(leftover), TestValue::Struct(without_id)],
        0,
    )
}

fn delivery_handler() -> TestValue {
    let delivery = delivery_contract();
    let bare = contract(
        CONTRACT_B,
        "Deliver_B",
        TEMPLATE,
        overrides(None, None, Vec::new()),
    )
    .value(
        "additionalPrerequisites",
        TestValue::Array(vec![locality(LOC_B)]),
    )
    .value(
        "subContracts",
        TestValue::Array(vec![
            TestValue::Struct(
                TestInstance::new("SubContract")
                    .value("id", TestValue::Guid(SUB_ONE + 1))
                    .value("debugName", TestValue::text("Test_Sub")),
            ),
            TestValue::Struct(TestInstance::new("SubContract")),
        ]),
    );
    let nested = contract(
        CONTRACT_D,
        "Deliver_D",
        TEMPLATE,
        overrides(
            Some("@title_nested"),
            None,
            vec![property("Wrap", string_hash("@wrap_text"))],
        ),
    );
    let brokered = TestInstance::new("CareerContract")
        .value("id", TestValue::Guid(CONTRACT_E))
        .value("debugName", TestValue::text("Haul_E"))
        .value("missionBrokerEntry", reference(BROKER));
    handler(
        "Delivery_Career",
        contractor_params("@contractor_acme"),
        vec![
            TestValue::Struct(delivery),
            TestValue::Struct(bare),
            TestValue::Struct(nested),
            TestValue::Struct(brokered),
        ],
        Vec::new(),
        FACTION,
    )
}

pub(super) fn generator(builder: DataForgeBuilder) -> DataForgeBuilder {
    builder.record(
        TestRecord::new(
            GENERATOR,
            "ContractGenerator",
            "ContractGenerator.Delivery",
            "libs/foundry/records/contracts/contractgenerator/delivery.xml",
        )
        .value(
            "generators",
            TestValue::Array(vec![delivery_handler(), bounty_handler()]),
        ),
    )
}

fn template(
    guid: u128,
    name: &str,
    title: &str,
    kind: u128,
    properties: Vec<TestValue>,
) -> TestRecord {
    TestRecord::new(
        guid,
        "ContractTemplate",
        format!("ContractTemplate.{name}"),
        format!("libs/foundry/records/contracts/contracttemplates/{name}.xml"),
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

pub(super) fn referenced(builder: DataForgeBuilder) -> DataForgeBuilder {
    let delivery_properties = vec![
        property("Cargo", string_hash("@cargo_gold")),
        property("Location", plain("MissionPropertyValue_Location")),
        property("Contractor", organization(ORGANIZATION)),
        property("Target", plain("MissionPropertyValue_AIName")),
        property("Ship", plain("MissionPropertyValue_ShipSpawnDescriptions")),
        property("Flag", plain("MissionPropertyValue_Boolean")),
    ];
    [
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
        mission_type(DELIVERY, "delivery", "@type_delivery"),
        mission_type(BOUNTY, "bounty", "@type_bounty"),
        mission_type(ESCORT, "escort", "@type_escort"),
        TestRecord::new(DATABASE, "TagDatabase", "TagDatabase.TagDatabase", TAG_PATH).value(
            "tags",
            TestValue::Array(vec![reference(TAG_TITLE), reference(TAG_OTHER)]),
        ),
        tag(TAG_TITLE, "Title"),
        tag(TAG_OTHER, "Other"),
        plain_record(
            ORGANIZATION,
            "MissionOrganization",
            "missiondata/pu_organizations/acme",
        )
        .value(
            "stringVariants",
            TestValue::Struct(TestInstance::new("MissionStringVariants").value(
                "variants",
                TestValue::Array(vec![
                    variant(TAG_OTHER, "@org_acme_other"),
                    variant(TAG_TITLE, "@org_acme_title"),
                ]),
            )),
        ),
    ]
    .into_iter()
    .fold(builder, DataForgeBuilder::record)
}
