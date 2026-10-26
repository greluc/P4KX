// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    CryXmlByteOrder, CryXmlSignature, TestElement, TestEntity, TestEntry, TestLoadout,
    TestLoadoutEntry,
};

const MALE_ARMOUR: &str = "Objects/Characters/Human/male_v7/armor/test";
const FEMALE_ARMOUR: &str = "Objects/Characters/Human/female_v2/armor/test";

/// The path of the XML Loadout of `TEST_Grunt`, as its Record names it.
pub const ARMOUR_XML_LOADOUT: &str = "Scripts/Loadouts/Grunt.xml";

pub(crate) fn loadout_entities() -> Vec<TestEntity> {
    let worn = |guid: u128, class: &str, item_type: &str| {
        TestEntity::new(guid, class, format!("entities/scitem/{class}.xml")).attach_type(item_type)
    };
    let torso = worn(0x10, "TEST_Torso", "Char_Armor_Torso")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_suit.skin"), None)
        .geometry_variant("Female", format!("{FEMALE_ARMOUR}/f_suit.skin"), None);
    let helmet = worn(0x11, "TEST_Helmet", "Char_Armor_Helmet").geometry_variant(
        "Male",
        format!("{MALE_ARMOUR}/m_hat.skin"),
        None,
    );
    let plate = worn(0x12, "TEST_Plate", "Char_Armor_Torso").geometry_variant(
        "Male",
        format!("{MALE_ARMOUR}/m_boots.skin"),
        None,
    );
    let pants = worn(0x14, "TEST_Pants", "Char_Clothing_Legs").geometry_variant(
        "Male",
        format!("{MALE_ARMOUR}/m_boots.skin"),
        None,
    );
    let cuirass = worn(0x15, "TEST_Cuirass", "Char_Armor_Torso").geometry_variant(
        "Male",
        format!("{MALE_ARMOUR}/m_hat.skin"),
        None,
    );
    let pistol = worn(0x13, "TEST_Pistol", "WeaponPersonal")
        .geometry(format!("{MALE_ARMOUR}/plain.skin"), None);
    let soldier = TestEntity::new(0x20, "TEST_Soldier", "entities/scitem/test_soldier.xml")
        .loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("torso", "TEST_Torso"),
            TestLoadoutEntry::class("weapon", "TEST_Pistol"),
            TestLoadoutEntry::class("helmet", "TEST_Helmet"),
            TestLoadoutEntry::class("lost", "TEST_Gone"),
            TestLoadoutEntry::empty("body_port").loadout(TestLoadout::Manual(vec![
                TestLoadoutEntry::class("legs", "TEST_Pants"),
            ])),
        ]));
    let grunt = TestEntity::new(0x21, "TEST_Grunt", "entities/scitem/test_grunt.xml")
        .loadout(TestLoadout::Xml(ARMOUR_XML_LOADOUT.to_owned()));
    let mut entities = vec![torso, helmet, plate, pants, cuirass, pistol, soldier, grunt];
    entities.extend(nested_entities(&worn));
    entities
}

fn nested_entities(worn: &impl Fn(u128, &str, &str) -> TestEntity) -> Vec<TestEntity> {
    let pouch = worn(0x16, "TEST_Pouch", "Char_Clothing_Hands").geometry_variant(
        "Male",
        format!("{MALE_ARMOUR}/m_boots.skin"),
        None,
    );
    let pack = worn(0x17, "TEST_Pack", "Char_Armor_Backpack")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_hat.skin"), None)
        .loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("pouch", "TEST_Pouch"),
            TestLoadoutEntry::class("spare", "TEST_Missing"),
        ]));
    let jacket = worn(0x18, "TEST_Jacket", "Char_Armor_Core")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_suit.skin"), None)
        .loadout(TestLoadout::Xml("Scripts/Loadouts/Missing.xml".to_owned()));
    let rifle = worn(0x19, "TEST_Rifle", "WeaponPersonal").loadout(TestLoadout::Manual(vec![
        TestLoadoutEntry::class("optic", "TEST_NoOptic"),
    ]));
    let ranger = TestEntity::new(0x22, "TEST_Ranger", "entities/scitem/test_ranger.xml").loadout(
        TestLoadout::Manual(vec![
            TestLoadoutEntry::class("weapon", "TEST_Rifle"),
            TestLoadoutEntry::class("pack", "TEST_Pack"),
            TestLoadoutEntry::class("jacket", "TEST_Jacket"),
        ]),
    );
    vec![pouch, pack, jacket, rifle, ranger]
}

/// Returns the XML Loadout of `TEST_Grunt`, with the path of the file it is stored at.
#[must_use]
pub fn armour_xml_loadout() -> (String, Vec<u8>) {
    let item = |port: &str, class: &str| {
        TestElement::new("Item")
            .attribute("portName", port)
            .attribute("itemName", class)
    };
    let loadout = TestElement::new("Loadout").child(
        TestElement::new("Items")
            .child(item("torso", "TEST_Torso"))
            .child(item("helmet", "TEST_Helmet")),
    );
    (
        ARMOUR_XML_LOADOUT.to_owned(),
        loadout.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
    )
}

pub(crate) fn loadout_files() -> Vec<TestEntry> {
    let (path, bytes) = armour_xml_loadout();
    vec![TestEntry::stored(
        format!("Data\\{}", path.replace('/', "\\")),
        bytes,
    )]
}
