use p4kx_testkit::{TestInstance, TestRecord, TestValue};

pub(super) fn single(name: &str, value: f32) -> (String, TestValue) {
    (name.to_owned(), TestValue::Single(value))
}

pub(super) fn instance(type_name: &str, values: Vec<(String, TestValue)>) -> TestValue {
    TestValue::Struct(
        values
            .into_iter()
            .fold(TestInstance::new(type_name), |instance, (name, value)| {
                instance.value(name, value)
            }),
    )
}

fn attach(kind: &str) -> TestValue {
    instance(
        "SAttachableComponentParams",
        vec![(
            "AttachDef".to_owned(),
            instance(
                "SItemDefinition",
                vec![("Type".to_owned(), TestValue::text(kind))],
            ),
        )],
    )
}

pub(super) fn entity(guid: u128, class: &str, kind: &str, extra: Vec<TestValue>) -> TestRecord {
    let mut components = vec![attach(kind)];
    components.extend(extra);
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/scitem/ships/{class}.xml"),
    )
    .value("Components", TestValue::Array(components))
}

pub(super) fn damage(physical: f32, energy: f32) -> TestValue {
    instance(
        "DamageInfo",
        vec![
            single("DamagePhysical", physical),
            single("DamageEnergy", energy),
        ],
    )
}
