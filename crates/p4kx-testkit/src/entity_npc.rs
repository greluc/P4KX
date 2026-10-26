// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_armour::armour_database;
use crate::entity_armour_loadout::ARMOUR_XML_LOADOUT;
use crate::entity_head::{MALE_HEAD_FOLDER, MALE_RIG, head_files, head_part, skin_entries};
use crate::{
    TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestMtl, TestMtlMaterial, TestPort,
};

/// The class name of the story character of [`npc_entries`].
pub const NPC_CLASS: &str = "TEST_Npc";
/// The class name of the actor of [`npc_entries`] whose base Loadout and outfits are all the XML
/// Loadout `Scripts/Loadouts/Grunt.xml`; its second outfit has the name tag `night`.
pub const NPC_FILES_CLASS: &str = "TEST_NpcFiles";
/// The folder of the character's own head skins below `Data`, with `/` as separator.
pub const NPC_HEAD_FOLDER: &str = "Objects/Characters/Human/heads/male/pu/npc_test";
/// The class name of the character's head, whose port flags are [`NPC_HEAD_FLAGS`].
pub const NPC_HEAD_CLASS: &str = "npc_test_t1_head";
/// The flags of the eyes port of the head, which select the sub-geometry tagged with them.
pub const NPC_HEAD_FLAGS: &str = "npc_test_t1_head";
/// The material library of the head, in [`NPC_HEAD_FOLDER`].
pub const NPC_HEAD_MATERIAL: &str =
    "Objects/Characters/Human/heads/male/pu/npc_test/npc_test_t1_head_material.mtl";
/// The material library of the eyes, in [`NPC_HEAD_FOLDER`].
pub const NPC_EYES_MATERIAL: &str =
    "Objects/Characters/Human/heads/male/pu/npc_test/npc_test_t1_eyes_material.mtl";
/// The material library of the male hair for the variant `hairVar_Brown`.
pub const NPC_HAIR_BROWN: &str = "Objects/Test/hair_brown.mtl";
/// The material library of the male hair for the variant `hairVar_Blonde`.
pub const NPC_HAIR_BLONDE: &str = "Objects/Test/hair_blonde.mtl";

/// The eyebrow skin in [`NPC_HEAD_FOLDER`] that the sub-geometry tagged with [`NPC_HEAD_FLAGS`]
/// names; the flags sit at the port container of the head, not at the eyebrow port.
pub const NPC_BROW_SKIN: &str =
    "Objects/Characters/Human/heads/male/pu/npc_test/npc_test_t1_eyebrow.skin";

const OTHER_BROW_SKIN: &str = "Objects/Test/other_eyebrow.skin";
const HAIR_SKIN: &str = "Objects/Test/hair";

fn windows(path: &str) -> String {
    format!("Data\\{}", path.replace('/', "\\"))
}

fn entry(port: &str, class: &str) -> TestLoadoutEntry {
    TestLoadoutEntry::class(port, class)
}

const fn manual(entries: Vec<TestLoadoutEntry>) -> TestLoadout {
    TestLoadout::Manual(entries)
}

fn head_entities() -> Vec<TestEntity> {
    let generic_head = format!("{MALE_HEAD_FOLDER}/{MALE_RIG}_head.skin");
    let generic_eyes = format!("{MALE_HEAD_FOLDER}/{MALE_RIG}_eyes.skin");
    let head = TestEntity::new(0x302, NPC_HEAD_CLASS, "entities/scitem/test_npc_head.xml")
        .attach_type("Char_Head")
        .sub_type("Male")
        .geometry(generic_head, Some(NPC_HEAD_MATERIAL))
        .port_flags(NPC_HEAD_FLAGS)
        .port(TestPort::new("Eyes_ItemPort").flags(NPC_HEAD_FLAGS))
        .port(TestPort::new("Eyebrow_ItemPort"))
        .port(TestPort::new("Hair_ItemPort"));
    let brow = TestEntity::new(0x308, "TEST_NpcBrow", "entities/scitem/test_npc_brow.xml")
        .attach_type("Char_Head_Eyebrow")
        .geometry(OTHER_BROW_SKIN, None::<&str>)
        .geometry_variant(NPC_HEAD_FLAGS, NPC_BROW_SKIN, None)
        .geometry_variant("other_head", OTHER_BROW_SKIN, None);
    let eyes = TestEntity::new(0x303, "TEST_NpcEyes", "entities/scitem/test_npc_eyes.xml")
        .attach_type("Char_Head_Eyes")
        .geometry_variant(
            NPC_HEAD_FLAGS,
            generic_eyes.clone(),
            Some(NPC_EYES_MATERIAL),
        )
        .geometry_variant("other_head", generic_eyes, None);
    let hair = TestEntity::new(0x304, "TEST_NpcHair", "entities/scitem/test_npc_hair.xml")
        .attach_type("Char_Head_Hair")
        .geometry_variant("Male", format!("{HAIR_SKIN}.skin"), None)
        .material_choice("hairVar_Brown>Male", NPC_HAIR_BROWN)
        .material_choice("hairVar_Blonde>Male", NPC_HAIR_BLONDE);
    let brown = TestEntity::new(
        0x305,
        "TEST_HairBrown",
        "entities/scitem/test_hair_brown.xml",
    )
    .attach_type("Char_Hair_Color")
    .tags("UnifiedHead $$hairVar_Brown++");
    vec![head, eyes, brow, hair, brown]
}

fn entities() -> Vec<TestEntity> {
    let body = TestEntity::new(0x301, "TEST_NpcBody", "entities/scitem/test_npc_body.xml")
        .attach_type("Char_Body");
    let shirt = TestEntity::new(0x306, "TEST_NpcShirt", "entities/scitem/test_npc_shirt.xml")
        .attach_type("Char_Clothing_Torso_0")
        .geometry_variant(
            "Male",
            "Objects/Characters/Human/male_v7/armor/test/m_suit.skin",
            None,
        );
    let tagged = TestEntity::new(0x307, "TEST_NpcCoat", "entities/scitem/test_npc_coat.xml")
        .attach_type("Char_Clothing_Torso_1")
        .geometry_variant(
            "Male",
            "Objects/Characters/Human/male_v7/armor/test/m_hat.skin",
            None,
        );
    let base = manual(vec![entry("Body_ItemPort", "TEST_NpcBody").loadout(
        manual(vec![entry("Head_ItemPort", NPC_HEAD_CLASS).loadout(
            manual(vec![
            entry("Eyes_ItemPort", "TEST_NpcEyes"),
            entry("Eyebrow_ItemPort", "TEST_NpcBrow"),
            entry("Hair_ItemPort", "TEST_NpcHair")
                .loadout(manual(vec![entry("Material_Variant", "TEST_HairBrown")])),
        ]),
        )]),
    )]);
    let clothing = |class: &str| {
        manual(vec![
            TestLoadoutEntry::empty("Body_ItemPort")
                .loadout(manual(vec![entry("Clothing_Torso_0", class)])),
        ])
    };
    let npc = TestEntity::new(0x300, NPC_CLASS, "entities/scitem/test_npc.xml").loadout(
        TestLoadout::Outfit {
            base: Box::new(base),
            outfits: vec![
                (Vec::new(), vec![clothing("TEST_NpcShirt")]),
                (vec!["tagged".to_owned()], vec![clothing("TEST_NpcCoat")]),
            ],
        },
    );
    let xml = || TestLoadout::Xml(ARMOUR_XML_LOADOUT.to_owned());
    let files = TestEntity::new(0x30a, NPC_FILES_CLASS, "entities/scitem/test_npc_files.xml")
        .loadout(TestLoadout::Outfit {
            base: Box::new(xml()),
            outfits: vec![
                (Vec::new(), vec![xml()]),
                (vec!["night".to_owned()], vec![xml()]),
            ],
        });
    let mut all = vec![body, shirt, tagged, npc, files];
    all.extend(head_entities());
    all
}

/// Returns the Entries of a synthetic Archive with a story character whose default Loadout is an
/// outfit, and the head it wears.
///
/// `TEST_Npc` has the base Loadout `Body_ItemPort` holding `TEST_NpcBody`, whose Loadout puts the
/// head [`NPC_HEAD_CLASS`] (`Char_Head`, `Male`) on `Head_ItemPort`, the eyes on its
/// `Eyes_ItemPort` (flags [`NPC_HEAD_FLAGS`]) and the hair, with the hair colour `hairVar_Brown`,
/// on its `Hair_ItemPort`; one outfit without name tags adds `TEST_NpcShirt` below an empty
/// `Body_ItemPort`, one with the tag `tagged` adds `TEST_NpcCoat`. The head's own skins for the
/// head and the eyes are in [`NPC_HEAD_FOLDER`], next to the material libraries; the generic
/// Protos skins are in the folder of the head rig. The hair has the material libraries
/// [`NPC_HAIR_BROWN`] and [`NPC_HAIR_BLONDE`] by the tags `hairVar_Brown` and `hairVar_Blonde`.
/// The bodies and the skins of [`armour_entries`](crate::armour_entries) are included.
#[must_use]
pub fn npc_entries() -> Vec<TestEntry> {
    let builder = entities()
        .iter()
        .fold(armour_database(), |builder, entity| {
            builder.record(entity.record())
        });
    let mut entries = vec![TestEntry::zstd("Data\\Game2.dcb", builder.build())];
    entries.extend(crate::entity_armour::armour_files());
    entries.extend(head_files(None));
    entries.extend(skin_entries(&head_part("eyes"), None));
    for part in ["head", "eyes"] {
        let stem = windows(&format!("{NPC_HEAD_FOLDER}/npc_test_t1_{part}"));
        entries.extend(skin_entries(&stem, None));
    }
    entries.extend(skin_entries(&windows(HAIR_SKIN), None));
    for skin in [NPC_BROW_SKIN, OTHER_BROW_SKIN] {
        entries.extend(skin_entries(&windows(&skin.replace(".skin", "")), None));
    }
    for path in [
        NPC_HEAD_MATERIAL,
        NPC_EYES_MATERIAL,
        NPC_HAIR_BROWN,
        NPC_HAIR_BLONDE,
    ] {
        let library = TestMtl::multi(&[TestMtlMaterial::new("one")]);
        entries.push(TestEntry::stored(windows(path), library.cryxml()));
    }
    entries
}
