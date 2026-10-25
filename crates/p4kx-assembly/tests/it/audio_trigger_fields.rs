// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Trigger discovery by field name: `GlobalResourceAudio.audioTrigger` alone, or every string
//! field whose name contains `audioTrigger`.

use std::error::Error;

use p4kx_assembly::Entities;
use p4kx_assembly::audio::{TriggerScope, trigger_references, trigger_references_in};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestInstance, TestValue, audio_schema, entity_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

fn melee() -> TestEntity {
    TestEntity::new(0x1, "TEST_Melee", "entities/test_melee.xml")
        .audio_trigger("Play_Slash")
        .component(TestValue::Struct(
            TestInstance::new("SMeleeAudioTestParams")
                .value("audioTriggerName", TestValue::text("Play_Swing"))
                .value("soundTag", TestValue::text("Play_Decoy")),
        ))
}

fn names(
    forge: &DataForge<'_>,
    scope: Option<TriggerScope>,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let tree = Entities::new(forge).find("TEST_Melee")?.tree()?;
    let references = scope.map_or_else(
        || trigger_references(&tree),
        |scope| trigger_references_in(&tree, scope),
    );
    Ok(references
        .iter()
        .map(|reference| (reference.name().to_owned(), reference.location().to_owned()))
        .collect())
}

#[test]
fn the_global_resource_audio_scope_ignores_other_structs() -> TestResult {
    let bytes = audio_schema(entity_schema(DataForgeBuilder::new()))
        .record(melee().record())
        .build();
    let forge = DataForge::open(&bytes)?;
    assert_eq!(
        names(&forge, Some(TriggerScope::GlobalResourceAudio))?,
        [(
            "Play_Slash".to_owned(),
            "Components[0].triggers[0].audioTrigger".to_owned()
        )]
    );
    Ok(())
}

#[test]
fn the_default_scope_adds_string_fields_named_like_audio_trigger() -> TestResult {
    let bytes = audio_schema(entity_schema(DataForgeBuilder::new()))
        .record(melee().record())
        .build();
    let forge = DataForge::open(&bytes)?;
    let expected = [
        (
            "Play_Slash".to_owned(),
            "Components[0].triggers[0].audioTrigger".to_owned(),
        ),
        (
            "Play_Swing".to_owned(),
            "Components[1].audioTriggerName".to_owned(),
        ),
    ];
    assert_eq!(names(&forge, None)?, expected, "default");
    assert_eq!(
        names(&forge, Some(TriggerScope::AudioTriggerFields))?,
        expected,
        "named scope; the soundTag field is no trigger"
    );
    Ok(())
}
