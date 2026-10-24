// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Finding an Entity by record path, GUID or class name.

use std::error::Error;

use p4kx_assembly::{Entities, EntityLookupError};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestRecord, TestStruct, entity_schema};

type TestResult = Result<(), Box<dyn Error>>;

fn database() -> DataForgeBuilder {
    let entities = [
        TestEntity::new(
            0x0079_c5d5_1678_4f8c_85ba_18ca_8f64_2af6,
            "AEGS_Avenger_Titan",
            "libs/foundry/records/entities/spaceships/aegs_avenger_titan.xml",
        ),
        TestEntity::new(0x1, "Twin", "libs/foundry/records/entities/twins/one.xml"),
        TestEntity::new(0x2, "Twin", "libs/foundry/records/entities/twins/two.xml"),
        TestEntity::new(0x3, "Shared_A", "libs/foundry/records/entities/shared.xml"),
        TestEntity::new(0x4, "Shared_B", "libs/foundry/records/entities/shared.xml"),
    ];
    entities
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
        .structure(TestStruct::new("Tag"))
        .record(TestRecord::new(
            0x9,
            "Tag",
            "Tag.Shiny",
            "libs/foundry/records/tags/shiny.xml",
        ))
}

fn with_entities(check: impl FnOnce(&Entities<'_>) -> TestResult) -> TestResult {
    let bytes = database().build();
    let forge = DataForge::open(&bytes)?;
    check(&Entities::new(&forge))
}

fn class_of(entities: &Entities<'_>, text: &str) -> Result<String, Box<dyn Error>> {
    Ok(entities.find(text)?.name()?.to_owned())
}

#[test]
fn an_entity_is_found_by_class_name_ignoring_case() -> TestResult {
    with_entities(|entities| {
        let expected = "EntityClassDefinition.AEGS_Avenger_Titan";
        assert_eq!(class_of(entities, "AEGS_Avenger_Titan")?, expected, "exact");
        assert_eq!(class_of(entities, "aegs_avenger_titan")?, expected, "case");
        assert_eq!(class_of(entities, expected)?, expected, "Record name");
        let lower = expected.to_ascii_lowercase();
        assert_eq!(
            class_of(entities, &lower)?,
            expected,
            "Record name in lower case"
        );
        Ok(())
    })
}

#[test]
fn an_entity_is_found_by_guid() -> TestResult {
    with_entities(|entities| {
        let found = class_of(entities, "0079c5d5-1678-4f8c-85ba-18ca8f642af6")?;
        assert_eq!(found, "EntityClassDefinition.AEGS_Avenger_Titan", "GUID");
        Ok(())
    })
}

#[test]
fn an_entity_is_found_by_record_path_in_any_spelling() -> TestResult {
    with_entities(|entities| {
        for path in [
            "libs/foundry/records/entities/spaceships/aegs_avenger_titan",
            "Libs\\Foundry\\Records\\Entities\\Spaceships\\AEGS_Avenger_Titan.xml",
        ] {
            assert_eq!(
                class_of(entities, path)?,
                "EntityClassDefinition.AEGS_Avenger_Titan",
                "{path}"
            );
        }
        Ok(())
    })
}

#[test]
fn several_matches_are_ambiguous_and_name_each_candidate() -> TestResult {
    with_entities(|entities| {
        for text in ["Twin", "libs/foundry/records/entities/shared"] {
            match entities.find(text) {
                Err(EntityLookupError::Ambiguous { candidates, .. }) => {
                    assert_eq!(candidates.len(), 2, "{text}: {candidates:?}");
                }
                other => return Err(format!("{text}: {other:?}").into()),
            }
        }
        Ok(())
    })
}

#[test]
fn no_match_and_a_record_of_another_type_are_errors() -> TestResult {
    with_entities(|entities| {
        assert!(
            matches!(
                entities.find("No_Such_Ship"),
                Err(EntityLookupError::NotFound { .. })
            ),
            "unknown class name"
        );
        assert!(
            matches!(
                entities.find("libs/foundry/records/tags/shiny"),
                Err(EntityLookupError::NotFound { .. })
            ),
            "a record path without Entity"
        );
        let error = entities
            .find("00000000-0000-0000-0000-000000000009")
            .err()
            .ok_or("found a Tag")?;
        assert_eq!(
            error.to_string(),
            "the Record 00000000-0000-0000-0000-000000000009 is a Tag, not an Entity",
            "a GUID of a Tag"
        );
        Ok(())
    })
}
