// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A default Loadout generated at random from loadout assortments resolves to the first
//! alternative of each group.

use std::collections::HashMap;
use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, LoadoutFileError, LoadoutFiles, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{
    CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, TestElement, TestEntity, TestLoadout,
    TestLoadoutEntry, assortment_record, entity_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

const ASSORTMENT: u128 = 0x100;
const MISSING_ASSORTMENT: u128 = 0x1ff;

struct Files(HashMap<String, Vec<u8>>);

impl LoadoutFiles for Files {
    fn read(&self, path: &str) -> Result<Vec<u8>, LoadoutFileError> {
        self.0
            .get(&path.to_ascii_lowercase())
            .cloned()
            .ok_or(LoadoutFileError::NotFound)
    }
}

fn body(class: &str) -> TestLoadout {
    TestLoadout::Manual(vec![TestLoadoutEntry::class("Body_ItemPort", class)])
}

fn resolve(loadout: TestLoadout, files: &Files) -> Result<EntityNode, Box<dyn Error>> {
    let builder = [
        TestEntity::new(0x1, "Beast", "entities/beast.xml").loadout(loadout),
        TestEntity::new(0x2, "Body_Red", "entities/body_red.xml"),
        TestEntity::new(0x3, "Body_Blue", "entities/body_blue.xml"),
        TestEntity::new(0x4, "Egg", "entities/egg.xml"),
    ]
    .iter()
    .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
        builder.record(entity.record())
    })
    .record(assortment_record(
        ASSORTMENT,
        "beast",
        &[
            ("Body", vec![body("Body_Red"), body("Body_Blue")]),
            (
                "Attachments",
                vec![TestLoadout::Xml("Loadouts/Egg.xml".to_owned())],
            ),
        ],
    ));
    let bytes = builder.build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge).with_files(files);
    let beast = resolver.entities().find("Beast")?;
    Ok(resolver.resolve(&beast)?)
}

fn egg_files() -> Files {
    let egg = TestElement::new("Loadout").child(
        TestElement::new("Items").child(
            TestElement::new("Item")
                .attribute("portName", "Egg_ItemPort")
                .attribute("itemName", "Egg"),
        ),
    );
    Files(HashMap::from([(
        "loadouts/egg.xml".to_owned(),
        egg.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
    )]))
}

fn attached(node: &EntityNode) -> Vec<(String, String)> {
    node.walk()
        .into_iter()
        .filter_map(|(path, port)| match port.attachment() {
            Attachment::Entity(child) => Some((path, child.class_name().to_owned())),
            _ => None,
        })
        .collect()
}

#[test]
fn the_first_alternative_of_each_group_is_attached() -> TestResult {
    let tree = resolve(TestLoadout::RandomGenerated(vec![ASSORTMENT]), &egg_files())?;

    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(
        attached(&tree),
        [
            ("Body_ItemPort".to_owned(), "Body_Red".to_owned()),
            ("Egg_ItemPort".to_owned(), "Egg".to_owned()),
        ],
        "the first body, not the second, and the group of the XML Loadout"
    );
    Ok(())
}

#[test]
fn an_assortment_without_a_record_is_an_issue_and_the_rest_still_resolves() -> TestResult {
    let tree = resolve(
        TestLoadout::RandomGenerated(vec![MISSING_ASSORTMENT, ASSORTMENT]),
        &egg_files(),
    )?;

    assert_eq!(
        tree.issues()
            .iter()
            .map(p4kx_assembly::LoadoutError::kind)
            .collect::<Vec<_>>(),
        ["dangling_reference"]
    );
    assert_eq!(attached(&tree).len(), 2);
    Ok(())
}
