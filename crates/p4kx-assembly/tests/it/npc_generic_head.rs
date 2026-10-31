// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A character whose head names no body (the generic head of most NPCs) is dressed by its
//! `Char_Body` Entity; a worn item whose only geometry is a world prop has no geometry.

use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, Geometry, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestPort, entity_schema};

type TestResult = Result<(), Box<dyn Error>>;

const BOX: &str = "Objects/buildingsets/human/universal/prop/storage/box_cardboard/box_a.cgf";
const JACKET_MALE: &str = "Objects/Characters/Human/male_v7/clothing/jacket_m.skin";
const JACKET_FEMALE: &str = "Objects/Characters/Human/female_v2/clothing/jacket_f.skin";
const SHIRT_SKIN: &str = "Objects/Characters/Human/male_v7/clothing/shirt.skin";
const FEMALE_BODY: &str = "Objects/Characters/Human/female_v2/body/f_body.skin";

const JACKET: u128 = 0x3000;
const SHIRT: u128 = 0x3100;
const UNIFIED: u128 = 0x3200;
const BOXED: u128 = 0x3300;
const HEAD_GENERIC: u128 = 0x2000;
const HEAD_FEMALE: u128 = 0x2100;
const BODY_MALE: u128 = 0x4000;
const BODY_FLUID: u128 = 0x4100;
const BODY_PLAIN: u128 = 0x4200;

fn worn(guid: u128, name: &str, item_type: &str) -> TestEntity {
    TestEntity::new(guid, name, format!("entities/{name}.xml")).attach_type(item_type)
}

fn character(guid: u128, class: &str, head: u128, body: u128) -> TestEntity {
    TestEntity::new(guid, class, format!("entities/{class}.xml"))
        .port(TestPort::new("body").default_item(body))
        .port(TestPort::new("head").default_item(head))
        .port(TestPort::new("torso").default_item(JACKET))
        .port(TestPort::new("shirt").default_item(SHIRT))
        .port(TestPort::new("unified").default_item(UNIFIED))
        .port(TestPort::new("boxed").default_item(BOXED))
}

fn resolve(class: &str) -> Result<EntityNode, Box<dyn Error>> {
    let jacket = worn(JACKET, "TEST_Jacket", "Char_Clothing_Torso_1")
        .geometry(BOX, None)
        .geometry_variant("Male", JACKET_MALE, None)
        .geometry_variant("Female", JACKET_FEMALE, None);
    let shirt = worn(SHIRT, "TEST_Shirt", "Char_Clothing_Torso_0").geometry(SHIRT_SKIN, None);
    let unified = worn(UNIFIED, "TEST_Unified", "Char_Clothing_Torso_2")
        .geometry(BOX, None)
        .geometry_variant("heldEntity", BOX, None);
    let boxed = worn(BOXED, "TEST_Boxed", "Char_Clothing_Legs").geometry(BOX, None);
    let generic = worn(HEAD_GENERIC, "TEST_HeadGeneric", "Char_Head").sub_type("UNDEFINED");
    let female_head = worn(HEAD_FEMALE, "TEST_HeadFemale", "Char_Head")
        .sub_type("Female")
        .geometry("objects/test/head.skin", None);
    let body_male = worn(BODY_MALE, "TEST_BodyMale", "Char_Body")
        .sub_type("Male")
        .geometry(FEMALE_BODY, None);
    let body_fluid = worn(BODY_FLUID, "TEST_BodyFluid", "Char_Body")
        .sub_type("Fluid")
        .geometry(FEMALE_BODY, None);
    let body_plain = worn(BODY_PLAIN, "TEST_BodyPlain", "Char_Body")
        .sub_type("Fluid")
        .geometry("Objects/Characters/Creatures/x/x.skin", None);
    let bytes = [
        jacket,
        shirt,
        unified,
        boxed,
        generic,
        female_head,
        body_male,
        body_fluid,
        body_plain,
        character(0x1000, "TEST_ByBodySubType", HEAD_GENERIC, BODY_MALE),
        character(0x1100, "TEST_ByBodyFolder", HEAD_GENERIC, BODY_FLUID),
        character(0x1200, "TEST_HeadWins", HEAD_FEMALE, BODY_MALE),
        character(0x1300, "TEST_BodyUnknown", HEAD_GENERIC, BODY_PLAIN),
    ]
    .into_iter()
    .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
        builder.record(entity.record())
    })
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
        .find(|candidate| candidate.name() == port)
        .ok_or_else(|| port.to_owned())?;
    match found.attachment() {
        Attachment::Entity(child) => Ok(child.geometry().map(|g| Geometry::path(g).to_owned())),
        other => Err(format!("{port} holds {other:?}").into()),
    }
}

#[test]
fn a_generic_head_leaves_the_body_to_the_sub_type_of_the_body_entity() -> TestResult {
    let tree = resolve("TEST_ByBodySubType")?;
    assert!(tree.character_body().is_none());
    assert_eq!(geometry_at(&tree, "torso")?.as_deref(), Some(JACKET_MALE));
    Ok(())
}

#[test]
fn a_body_entity_with_another_sub_type_is_told_by_the_folder_of_its_model() -> TestResult {
    let tree = resolve("TEST_ByBodyFolder")?;
    assert_eq!(geometry_at(&tree, "torso")?.as_deref(), Some(JACKET_FEMALE));
    Ok(())
}

#[test]
fn a_head_that_names_a_body_decides_before_the_body_entity() -> TestResult {
    let tree = resolve("TEST_HeadWins")?;
    assert_eq!(geometry_at(&tree, "torso")?.as_deref(), Some(JACKET_FEMALE));
    Ok(())
}

#[test]
fn a_character_whose_head_and_body_name_no_body_keeps_the_main_geometry() -> TestResult {
    let tree = resolve("TEST_BodyUnknown")?;
    assert_eq!(geometry_at(&tree, "torso")?.as_deref(), Some(BOX));
    Ok(())
}

#[test]
fn worn_items_with_only_a_world_prop_have_no_geometry_once_the_body_is_known() -> TestResult {
    let tree = resolve("TEST_ByBodySubType")?;
    assert_eq!(geometry_at(&tree, "unified")?, None);
    assert_eq!(geometry_at(&tree, "boxed")?, None);
    Ok(())
}

#[test]
fn a_worn_item_with_a_character_model_and_no_variants_keeps_it() -> TestResult {
    let tree = resolve("TEST_ByBodySubType")?;
    assert_eq!(geometry_at(&tree, "shirt")?.as_deref(), Some(SHIRT_SKIN));
    Ok(())
}

#[test]
fn the_body_itself_keeps_its_geometry() -> TestResult {
    let tree = resolve("TEST_ByBodySubType")?;
    assert_eq!(geometry_at(&tree, "body")?.as_deref(), Some(FEMALE_BODY));
    Ok(())
}
