// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    DataForgeBuilder, TestEntity, TestEntry, TestModel, TestMtl, TestMtlMaterial, entity_schema,
};

/// The class name of the Entity of [`bone_thruster_entries`].
pub const BONE_THRUSTER: &str = "TEST_Thruster";

/// Returns the character definition of the thruster: its nozzle skin and the attachments
/// `attachments`, XML `Attachment` elements.
#[must_use]
pub fn bone_thruster_definition(attachments: &str) -> String {
    format!(
        r#"<CharacterDefinition>
  <AttachmentList>
    <Attachment Type="CA_SKIN" AName="Nozzle" Binding="Objects\Test\nozzle.skin"/>
    {attachments}
  </AttachmentList>
</CharacterDefinition>"#
    )
}

/// Returns the `Attachment` element of the thruster's housing at the bone `bone`, offset by 1 m
/// along X, bound to the file `binding`.
#[must_use]
pub fn bone_thruster_housing(bone: &str, binding: &str) -> String {
    format!(
        r#"<Attachment Type="CA_BONE" AName="Housing" BoneName="{bone}" Binding="{binding}"
      Material="Objects\Test\paint.mtl" RelPosition="1,0,0" RelRotation="1,0,0,0"/>"#
    )
}

/// Returns the Archive entries of [`BONE_THRUSTER`], whose geometry is the character definition
/// `definition`.
///
/// The Archive holds a two-bone nozzle skin (bones `root` and `tip`), a cube housing `housing.cgf`
/// and the material library `paint`.
#[must_use]
pub fn bone_thruster_entries(definition: &str) -> Vec<TestEntry> {
    let thruster = TestEntity::new(0x1, BONE_THRUSTER, "entities/test_thruster.xml")
        .geometry("Objects/Test/thruster.cdf", None);
    let database = entity_schema(DataForgeBuilder::new()).record(thruster.record());
    let nozzle = TestModel::two_bone_skin().with_material_library("Objects/Test/paint");
    let housing = TestModel::cube().with_material_library("Objects/Test/paint");
    let material = TestMtlMaterial::new("paint").diffuse([0.2, 0.4, 0.6]);
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database.build()),
        TestEntry::stored(
            "Data\\Objects\\Test\\thruster.cdf",
            definition.as_bytes().to_vec(),
        ),
        TestEntry::stored("Data\\Objects\\Test\\nozzle.skin", nozzle.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\nozzle.skinm", nozzle.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\housing.cgf", housing.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\housing.cgfm", housing.cgfm()),
        TestEntry::stored(
            "Data\\Objects\\Test\\paint.mtl",
            TestMtl::multi(&[material.clone(), material.clone(), material]).cryxml(),
        ),
    ]
}
