// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Which sub-geometry a face part takes when the head's flags and the body both name one.

use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, Geometry, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestPort, entity_schema};

const MAN: u128 = 0x1000;
const HEAD: u128 = 0x2000;
const LASHES: u128 = 0x3000;
const RACK: u128 = 0x4000;
const OWN: &str = "Objects/Test/lashes_own.skin";
const GENERIC: &str = "Objects/Test/lashes_generic.skin";
const FLAG: &str = "test_t1_head";

fn lashes() -> TestEntity {
    TestEntity::new(LASHES, "TEST_Lashes", "entities/lashes.xml")
        .attach_type("Char_Head_Eyelashes")
        .geometry_variant(FLAG, OWN, None)
        .geometry_variant("Male", GENERIC, None)
}

fn man() -> TestEntity {
    TestEntity::new(MAN, "TEST_Man", "entities/man.xml")
        .port(TestPort::new("head").default_item(HEAD))
        .port(TestPort::new("rack").default_item(RACK))
}

fn resolve(head: &TestEntity, rack: &TestEntity) -> Result<EntityNode, Box<dyn Error>> {
    let bytes = [lashes(), head.clone(), rack.clone(), man()]
        .into_iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let entity = resolver.entities().find("TEST_Man")?;
    Ok(resolver.resolve(&entity)?)
}

fn head(flags: Option<&str>) -> TestEntity {
    let head = TestEntity::new(HEAD, "TEST_Head", "entities/head.xml")
        .attach_type("Char_Head")
        .sub_type("Male")
        .port(TestPort::new("lashes").default_item(LASHES));
    match flags {
        Some(flags) => head.port_flags(flags),
        None => head,
    }
}

fn rack(flags: &str) -> TestEntity {
    TestEntity::new(RACK, "TEST_Rack", "entities/rack.xml")
        .attach_type("Char_Clothing_Torso_1")
        .port_flags(flags)
        .port(TestPort::new("lashes").default_item(LASHES))
}

fn lashes_path<'n>(tree: &'n EntityNode, owner: &str) -> Result<&'n str, Box<dyn Error>> {
    let node = |node: &'n EntityNode, port: &str| -> Option<&'n EntityNode> {
        node.ports()
            .iter()
            .find(|candidate| candidate.name() == port)
            .and_then(|found| match found.attachment() {
                Attachment::Entity(entity) => Some(&**entity),
                _ => None,
            })
    };
    let owner = node(tree, owner).ok_or("no owner")?;
    let lashes = node(owner, "lashes").ok_or("no lashes")?;
    Ok(lashes.geometry().map(Geometry::path).ok_or("no geometry")?)
}

#[test]
fn the_head_flags_win_over_the_body_tag() -> Result<(), Box<dyn Error>> {
    let tree = resolve(&head(Some(FLAG)), &rack(FLAG))?;
    assert_eq!(lashes_path(&tree, "head")?, OWN);
    Ok(())
}

#[test]
fn the_body_tag_chooses_when_the_head_flags_name_no_sub_geometry() -> Result<(), Box<dyn Error>> {
    let tree = resolve(&head(Some("other_head")), &rack(FLAG))?;
    assert_eq!(lashes_path(&tree, "head")?, GENERIC);
    Ok(())
}

#[test]
fn the_flags_of_a_container_that_is_no_head_do_not_choose() -> Result<(), Box<dyn Error>> {
    let tree = resolve(&head(Some(FLAG)), &rack(FLAG))?;
    assert_eq!(lashes_path(&tree, "rack")?, GENERIC);
    Ok(())
}
