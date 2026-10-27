// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The clothing of a character takes the sub-geometry of the character's body, not its main
//! geometry.

use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestPort, entity_schema};

type TestResult = Result<(), Box<dyn Error>>;

const MAN: u128 = 0x1000;
const WOMAN: u128 = 0x1100;
const HEAD_MALE: u128 = 0x2000;
const HEAD_FEMALE: u128 = 0x2100;
const JACKET: u128 = 0x3000;
const SHIRT: u128 = 0x3100;
const BOOTS: u128 = 0x3200;
const HAT: u128 = 0x3300;
const HOOD: u128 = 0x3400;
const GLASS: u128 = 0x3500;

fn character(guid: u128, class: &str, head: u128) -> TestEntity {
    TestEntity::new(guid, class, format!("entities/{class}.xml"))
        .port(TestPort::new("head").default_item(head))
        .port(TestPort::new("torso").default_item(JACKET))
        .port(TestPort::new("shirt").default_item(SHIRT))
        .port(TestPort::new("feet").default_item(BOOTS))
        .port(TestPort::new("hat").default_item(HAT))
        .port(TestPort::new("hood").flags("up").default_item(HOOD))
        .port(TestPort::new("glass").default_item(GLASS))
}

fn resolve(class: &str) -> Result<EntityNode, Box<dyn Error>> {
    let head = |guid, name: &str, sub: &str| {
        TestEntity::new(guid, name, format!("entities/{name}.xml"))
            .attach_type("Char_Head")
            .sub_type(sub)
            .geometry("objects/test/head.skin", None)
    };
    let worn = |guid, name: &str, item_type: &str| {
        TestEntity::new(guid, name, format!("entities/{name}.xml")).attach_type(item_type)
    };
    let jacket = worn(JACKET, "TEST_Jacket", "Char_Clothing_Torso_1")
        .geometry("objects/test/box.cgf", None)
        .geometry_variant("Male", "objects/test/jacket_m.skin", None)
        .geometry_variant("Female", "objects/test/jacket_f.skin", None);
    let shirt = worn(SHIRT, "TEST_Shirt", "Char_Clothing_Torso_0")
        .geometry("objects/test/shirt.skin", None);
    let boots = worn(BOOTS, "TEST_Boots", "Char_Clothing_Feet")
        .geometry("objects/test/box.cgf", None)
        .geometry_variant("Male", "objects/test/boots_m.skin", None);
    let hat = worn(HAT, "TEST_Hat", "Char_Clothing_Hat")
        .geometry_variant("Male", "objects/test/hat_m.skin", None)
        .geometry_variant("Female", "objects/test/hat_f.skin", None);
    let hood = worn(HOOD, "TEST_Hood", "Char_Clothing_Hat")
        .geometry_variant("up", "objects/test/hood_up.skin", None)
        .geometry_variant("Male", "objects/test/hood_m.skin", None);
    let glass = worn(GLASS, "TEST_Glass", "MobiGlas")
        .geometry("objects/test/glass_prop.skin", None)
        .geometry_variant("Male", "objects/test/glass_worn.skin", None);
    let bytes = entity_schema(DataForgeBuilder::new())
        .record(head(HEAD_MALE, "TEST_HeadMale", "Male").record())
        .record(head(HEAD_FEMALE, "TEST_HeadFemale", "Female").record())
        .record(jacket.record())
        .record(shirt.record())
        .record(boots.record())
        .record(hat.record())
        .record(hood.record())
        .record(glass.record())
        .record(character(MAN, "TEST_Man", HEAD_MALE).record())
        .record(character(WOMAN, "TEST_Woman", HEAD_FEMALE).record())
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let entity = resolver.entities().find(class)?;
    Ok(resolver.resolve(&entity)?)
}

fn geometry_at(node: &EntityNode, port: &str) -> Result<Option<String>, Box<dyn Error>> {
    let found = node
        .ports()
        .iter()
        .find(|p| p.name() == port)
        .ok_or_else(|| port.to_owned())?;
    match found.attachment() {
        Attachment::Entity(child) => Ok(child.geometry().map(|g| g.path().to_owned())),
        other => Err(format!("{port} holds {other:?}").into()),
    }
}

#[test]
fn clothing_takes_the_sub_geometry_of_the_body_its_head_names() -> TestResult {
    let man = resolve("TEST_Man")?;
    assert_eq!(
        geometry_at(&man, "torso")?.as_deref(),
        Some("objects/test/jacket_m.skin")
    );
    let woman = resolve("TEST_Woman")?;
    assert_eq!(
        geometry_at(&woman, "torso")?.as_deref(),
        Some("objects/test/jacket_f.skin")
    );
    Ok(())
}

#[test]
fn clothing_without_geometry_per_body_keeps_its_main_geometry() -> TestResult {
    let man = resolve("TEST_Man")?;
    assert_eq!(
        geometry_at(&man, "shirt")?.as_deref(),
        Some("objects/test/shirt.skin")
    );
    Ok(())
}

#[test]
fn clothing_with_no_sub_geometry_for_the_body_has_no_geometry_instead_of_the_main_one() -> TestResult
{
    let man = resolve("TEST_Man")?;
    assert_eq!(
        geometry_at(&man, "feet")?.as_deref(),
        Some("objects/test/boots_m.skin")
    );
    let woman = resolve("TEST_Woman")?;
    assert_eq!(geometry_at(&woman, "feet")?, None);
    Ok(())
}

#[test]
fn an_entity_with_only_sub_geometry_is_drawn_for_its_body() -> TestResult {
    let woman = resolve("TEST_Woman")?;
    assert_eq!(
        geometry_at(&woman, "hat")?.as_deref(),
        Some("objects/test/hat_f.skin")
    );
    Ok(())
}

#[test]
fn a_port_flag_keeps_its_priority_over_the_body() -> TestResult {
    let man = resolve("TEST_Man")?;
    assert_eq!(
        geometry_at(&man, "hood")?.as_deref(),
        Some("objects/test/hood_up.skin")
    );
    Ok(())
}

#[test]
fn an_item_that_is_no_character_item_keeps_its_main_geometry() -> TestResult {
    let man = resolve("TEST_Man")?;
    assert_eq!(
        geometry_at(&man, "glass")?.as_deref(),
        Some("objects/test/glass_prop.skin")
    );
    Ok(())
}
