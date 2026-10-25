// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A geometry that is a character definition (`.cdf`) draws its skin and reports the rest.

use crate::assembly_support::{TestResult, assemble, fixture};

use p4kx_assembly::IssueKind;
use p4kx_extraction::Archive;
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestEntry, TestModel, TestMtl, TestMtlMaterial, entity_schema,
};

const CHARACTER: &str = r#"<CharacterDefinition>
  <Model File="Objects\Test\rig_CHR.chr" LODRatio="40"/>
  <AttachmentList>
    <Attachment Type="CA_BONE" AName="Housing" BoneName="housing" Binding="Objects\Test\housing.cga"/>
    <Attachment Type="CA_SKIN" AName="Nozzle" Binding="Objects\Test\nozzle.cgf" Material="Objects/Test/paint.mtl"/>
  </AttachmentList>
</CharacterDefinition>"#;

fn entries(definition: &str) -> Vec<TestEntry> {
    let thruster = TestEntity::new(0x1, "TEST_Thruster", "entities/test_thruster.xml")
        .geometry("Objects/Test/thruster.cdf", None);
    let database = entity_schema(DataForgeBuilder::new()).record(thruster.record());
    let model = TestModel::cube();
    let material = TestMtlMaterial::new("paint").diffuse([0.2, 0.4, 0.6]);
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database.build()),
        TestEntry::stored(
            "Data\\Objects\\Test\\thruster.cdf",
            definition.as_bytes().to_vec(),
        ),
        TestEntry::stored("Data\\Objects\\Test\\nozzle.cgf", model.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\nozzle.cgfm", model.cgfm()),
        TestEntry::stored(
            "Data\\Objects\\Test\\paint.mtl",
            TestMtl::multi(&[material.clone(), material.clone(), material]).cryxml(),
        ),
    ]
}

#[test]
fn the_skin_of_a_character_definition_is_the_model_of_the_part() -> TestResult {
    let fixture = fixture(entries(CHARACTER))?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, "TEST_Thruster", 0)?;

    let root = assembly.root().ok_or("a root")?;
    assert!(root.model().is_some(), "{:?}", assembly.issues());
    assert_eq!(
        root.materials()
            .library()
            .map(|library| library.path.as_str()),
        Some("Objects/Test/paint"),
        "the attachment's material replaces the model's library"
    );
    let codes: Vec<(&str, &str)> = assembly
        .issues()
        .iter()
        .map(|issue| (issue.path(), issue.kind().code()))
        .collect();
    assert_eq!(codes, [("", "attachments_left_out")]);
    let IssueKind::AttachmentsLeftOut { files } = assembly.issues()[0].kind() else {
        return Err("the kind".into());
    };
    assert_eq!(files, &["Objects\\Test\\housing.cga".to_owned()]);
    assert!(!assembly.issues()[0].kind().skips());
    Ok(())
}

#[test]
fn a_character_definition_that_names_nothing_is_unreadable() -> TestResult {
    let fixture = fixture(entries("<CharacterDefinition/>"))?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, "TEST_Thruster", 0)?;

    assert!(assembly.root().ok_or("a root")?.model().is_none());
    assert_eq!(
        assembly
            .issues()
            .iter()
            .map(|issue| issue.kind().code())
            .collect::<Vec<_>>(),
        ["geometry_unreadable"]
    );
    Ok(())
}
