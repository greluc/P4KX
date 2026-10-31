// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A default Loadout generated from loadout assortments resolves to the first alternative of each
//! group, for any kind of Entity.

use std::collections::HashMap;
use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, LoadoutFileError, LoadoutFiles, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{
    CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, TestElement, TestEntity, TestLoadout,
    TestLoadoutEntry, TestRecord, TestRequirement, assortment_record, assortment_record_requiring,
    entity_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

const ASSORTMENT: u128 = 0x100;
const OTHER: u128 = 0x101;
const MISSING: u128 = 0x1ff;
const BEAST: u128 = 0x1;

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

fn generated(assortments: &[u128]) -> TestLoadout {
    TestLoadout::RandomGenerated(assortments.to_vec())
}

/// The assortment of a beast: a body group with two alternatives and an XML group.
fn beast_assortment() -> TestRecord {
    assortment_record(
        ASSORTMENT,
        "beast",
        &[
            ("Body", vec![body("Body_Red"), body("Body_Blue")]),
            (
                "Attachments",
                vec![TestLoadout::Xml("Loadouts/Egg.xml".to_owned())],
            ),
        ],
    )
}

fn resolve(
    loadout: TestLoadout,
    files: Option<&Files>,
    records: Vec<TestRecord>,
) -> Result<EntityNode, Box<dyn Error>> {
    let builder = [
        TestEntity::new(BEAST, "Beast", "entities/beast.xml").loadout(loadout),
        TestEntity::new(0x2, "Body_Red", "entities/body_red.xml"),
        TestEntity::new(0x3, "Body_Blue", "entities/body_blue.xml"),
        TestEntity::new(0x4, "Egg", "entities/egg.xml"),
    ]
    .iter()
    .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
        builder.record(entity.record())
    });
    let bytes = records
        .into_iter()
        .fold(builder, DataForgeBuilder::record)
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    if let Some(files) = files {
        resolver = resolver.with_files(files);
    }
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

fn kinds(node: &EntityNode) -> Vec<&'static str> {
    node.issues()
        .iter()
        .map(p4kx_assembly::LoadoutError::kind)
        .collect()
}

fn pair(port: &str, class: &str) -> (String, String) {
    (port.to_owned(), class.to_owned())
}

#[test]
fn the_first_alternative_of_each_group_is_attached() -> TestResult {
    let tree = resolve(
        generated(&[ASSORTMENT]),
        Some(&egg_files()),
        vec![beast_assortment()],
    )?;

    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(
        attached(&tree),
        [
            pair("Body_ItemPort", "Body_Red"),
            pair("Egg_ItemPort", "Egg")
        ],
        "the first body, not the second, and the group of the XML Loadout"
    );
    Ok(())
}

#[test]
fn an_assortment_without_a_record_is_an_issue_and_the_rest_still_resolves() -> TestResult {
    let tree = resolve(
        generated(&[MISSING, ASSORTMENT]),
        Some(&egg_files()),
        vec![beast_assortment()],
    )?;

    assert_eq!(kinds(&tree), ["dangling_reference"]);
    assert_eq!(attached(&tree).len(), 2);
    Ok(())
}

#[test]
fn a_record_that_is_no_assortment_is_an_issue() -> TestResult {
    let tree = resolve(generated(&[BEAST]), None, Vec::new())?;

    assert_eq!(kinds(&tree), ["assortment"]);
    assert!(tree.issues()[0].to_string().contains("not an assortment"));
    assert_eq!(attached(&tree), Vec::<(String, String)>::new());
    Ok(())
}

#[test]
fn an_assortment_that_names_itself_is_an_issue_and_ends() -> TestResult {
    let looping = assortment_record(
        ASSORTMENT,
        "loop",
        &[("Body", vec![generated(&[ASSORTMENT])])],
    );
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![looping])?;

    assert_eq!(kinds(&tree), ["assortment"]);
    assert!(tree.issues()[0].to_string().contains("names itself"));
    Ok(())
}

#[test]
fn two_assortments_that_name_each_other_are_an_issue_and_end() -> TestResult {
    let a = assortment_record(
        ASSORTMENT,
        "a",
        &[
            ("Body", vec![generated(&[OTHER])]),
            ("Own", vec![body("Body_Red")]),
        ],
    );
    let b = assortment_record(OTHER, "b", &[("Body", vec![generated(&[ASSORTMENT])])]);
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![a, b])?;

    assert_eq!(kinds(&tree), ["assortment"]);
    assert_eq!(attached(&tree), [pair("Body_ItemPort", "Body_Red")]);
    Ok(())
}

#[test]
fn an_empty_group_gives_nothing_and_no_issue() -> TestResult {
    let record = assortment_record(
        ASSORTMENT,
        "empty",
        &[("Nothing", Vec::new()), ("Body", vec![body("Body_Blue")])],
    );
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;

    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(attached(&tree), [pair("Body_ItemPort", "Body_Blue")]);
    Ok(())
}

#[test]
fn an_unreadable_first_alternative_is_an_issue_and_the_later_ones_are_not_tried() -> TestResult {
    let record = assortment_record(
        ASSORTMENT,
        "unreadable",
        &[(
            "Body",
            vec![TestLoadout::Xml(String::new()), body("Body_Blue")],
        )],
    );
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;

    assert_eq!(kinds(&tree), ["assortment"]);
    assert!(tree.issues()[0].to_string().contains("first alternative"));
    assert_eq!(attached(&tree), Vec::<(String, String)>::new());
    Ok(())
}

#[test]
fn a_later_assortment_replaces_the_port_of_an_earlier_one() -> TestResult {
    let second = assortment_record(OTHER, "second", &[("Body", vec![body("Body_Blue")])]);
    let tree = resolve(
        generated(&[ASSORTMENT, OTHER]),
        Some(&egg_files()),
        vec![beast_assortment(), second],
    )?;

    assert_eq!(
        attached(&tree),
        [
            pair("Body_ItemPort", "Body_Blue"),
            pair("Egg_ItemPort", "Egg")
        ]
    );
    Ok(())
}

const TAG: u128 = 0x500;

fn tagged() -> TestRequirement {
    TestRequirement::Tags {
        required: vec![TAG],
        banned: Vec::new(),
    }
}

/// An assortment of two groups with the Requirements `first` and `second`.
fn requiring(
    assortment: Option<&TestRequirement>,
    first: Option<TestRequirement>,
    second: Option<TestRequirement>,
) -> TestRecord {
    assortment_record_requiring(
        ASSORTMENT,
        "required",
        assortment,
        &[
            ("Body", first, vec![body("Body_Red")]),
            ("Legs", second, vec![body("Body_Blue")]),
        ],
    )
}

#[test]
fn a_requirement_that_restricts_nothing_applies_without_an_issue() -> TestResult {
    let record = requiring(
        Some(&TestRequirement::unrestricted()),
        Some(TestRequirement::unrestricted()),
        Some(TestRequirement::AnyOf(vec![
            tagged(),
            TestRequirement::unrestricted(),
        ])),
    );
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;

    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(attached(&tree), [pair("Body_ItemPort", "Body_Blue")]);
    Ok(())
}

#[test]
fn a_group_that_depends_on_tags_still_applies_and_is_listed() -> TestResult {
    let record = requiring(None, Some(tagged()), None);
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;

    assert_eq!(kinds(&tree), ["assortment_requirement"]);
    let reason = tree.issues()[0].to_string();
    assert!(reason.contains("the group Body"), "{reason}");
    assert_eq!(attached(&tree), [pair("Body_ItemPort", "Body_Blue")]);
    Ok(())
}

#[test]
fn an_assortment_that_depends_on_tags_still_applies_and_is_listed() -> TestResult {
    let record = requiring(Some(&tagged()), None, None);
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;

    assert_eq!(kinds(&tree), ["assortment_requirement"]);
    assert!(tree.issues()[0].to_string().contains("the assortment"));
    assert_eq!(attached(&tree).len(), 1);
    Ok(())
}

#[test]
fn a_group_whose_requirement_fails_gives_nothing() -> TestResult {
    let never = TestRequirement::Not(Box::new(TestRequirement::unrestricted()));
    let record = assortment_record_requiring(
        ASSORTMENT,
        "never",
        None,
        &[
            ("Body", Some(never), vec![body("Body_Red")]),
            (
                "Legs",
                None,
                vec![TestLoadout::Manual(vec![TestLoadoutEntry::class(
                    "Legs_ItemPort",
                    "Body_Blue",
                )])],
            ),
        ],
    );
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;

    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(attached(&tree), [pair("Legs_ItemPort", "Body_Blue")]);
    Ok(())
}

#[test]
fn an_assortment_whose_requirement_fails_gives_nothing() -> TestResult {
    let never = TestRequirement::Not(Box::new(TestRequirement::unrestricted()));
    let record = requiring(Some(&never), None, None);
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;

    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(attached(&tree), Vec::<(String, String)>::new());
    Ok(())
}

#[test]
fn not_and_or_follow_their_parts_and_an_empty_or_is_undecided() -> TestResult {
    let cases = [
        (TestRequirement::Not(Box::new(tagged())), "undecided"),
        (TestRequirement::AnyOf(Vec::new()), "undecided"),
        (
            TestRequirement::AnyOf(vec![TestRequirement::Not(Box::new(
                TestRequirement::unrestricted(),
            ))]),
            "unmet",
        ),
        (
            TestRequirement::Not(Box::new(TestRequirement::AnyOf(vec![
                tagged(),
                TestRequirement::unrestricted(),
            ]))),
            "unmet",
        ),
    ];
    for (requirement, expected) in cases {
        let record = requiring(None, Some(requirement.clone()), None);
        let tree = resolve(generated(&[ASSORTMENT]), None, vec![record])?;
        let outcome = if tree.issues().is_empty() {
            "unmet"
        } else {
            "undecided"
        };
        assert_eq!(outcome, expected, "{requirement:?}");
    }
    Ok(())
}

#[test]
fn an_xml_alternative_without_an_archive_is_an_issue() -> TestResult {
    let tree = resolve(generated(&[ASSORTMENT]), None, vec![beast_assortment()])?;

    assert_eq!(kinds(&tree), ["no_loadout_files"]);
    assert_eq!(attached(&tree), [pair("Body_ItemPort", "Body_Red")]);
    Ok(())
}
