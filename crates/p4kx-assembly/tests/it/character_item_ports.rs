// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! An item port of a character definition (`.cdf`) is anchored to a bone of its skeleton.

use crate::assembly_support::{TestResult, assemble, fixture};

use p4kx_assembly::Placement;
use p4kx_extraction::Archive;
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestModel, TestPort,
    entity_schema,
};

const ARM: &str = "TEST_Arm";

const CHARACTER: &str = r#"<CharacterDefinition>
  <AttachmentList>
    <Attachment Type="CA_SKIN" AName="Arm" Binding="Objects\Test\arm.skin"/>
  </AttachmentList>
</CharacterDefinition>"#;

fn entries() -> Vec<TestEntry> {
    let arm = TestEntity::new(0x1, ARM, "entities/test_arm.xml")
        .geometry("Objects/Test/arm.cdf", None)
        .port(TestPort::new("on_tip").helper("TIP", [0.0; 3], [0.0; 3]))
        .port(TestPort::new("on_root").helper("root", [1.0, 0.0, 0.0], [0.0; 3]))
        .port(TestPort::new("nowhere").helper("missing", [0.0; 3], [0.0; 3]))
        .loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("on_tip", "TEST_Laser"),
            TestLoadoutEntry::class("on_root", "TEST_Laser"),
            TestLoadoutEntry::class("nowhere", "TEST_Laser"),
        ]));
    let laser = TestEntity::new(0x2, "TEST_Laser", "entities/test_laser.xml")
        .geometry("Objects/Test/laser.cgf", None);
    let database = [arm, laser]
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        });
    let skin = TestModel::two_bone_skin();
    let laser = TestModel::cube();
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database.build()),
        TestEntry::stored(
            "Data\\Objects\\Test\\arm.cdf",
            CHARACTER.as_bytes().to_vec(),
        ),
        TestEntry::stored("Data\\Objects\\Test\\arm.skin", skin.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\arm.skinm", skin.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\laser.cgf", laser.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\laser.cgfm", laser.cgfm()),
    ]
}

#[test]
fn a_bone_of_the_skeleton_anchors_the_item_port() -> TestResult {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, ARM, 0)?;

    let laser = assembly.part_at("on_tip").ok_or("the part")?;
    assert_eq!(laser.placement(), &Placement::Helper("TIP".to_owned()));
    assert_eq!(laser.world().translation(), [0.0, 0.0, 2.0]);
    Ok(())
}

#[test]
fn the_offset_of_the_port_applies_in_the_bone_space() -> TestResult {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, ARM, 0)?;

    let laser = assembly.part_at("on_root").ok_or("the part")?;
    assert_eq!(laser.placement(), &Placement::Helper("root".to_owned()));
    assert_eq!(laser.world().translation(), [1.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn a_helper_that_is_no_bone_is_still_listed() -> TestResult {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, ARM, 0)?;

    let part = assembly.part_at("nowhere").ok_or("the part")?;
    assert_eq!(part.placement(), &Placement::Origin);
    let helpers: Vec<&str> = assembly
        .issues()
        .iter()
        .filter(|issue| issue.kind().code() == "helper_not_found")
        .map(p4kx_assembly::Issue::path)
        .collect();
    assert_eq!(helpers, ["nowhere"]);
    Ok(())
}
