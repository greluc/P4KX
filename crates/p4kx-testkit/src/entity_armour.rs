// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_armour_loadout::{loadout_entities, loadout_files};
use crate::model::TestPositions;
use crate::{
    DataForgeBuilder, TestBone, TestEntity, TestEntry, TestModel, TestMtl, TestMtlMaterial,
    TestSkeleton, entity_schema,
};

const MALE_SKELETON: &str = "Data\\Objects\\Characters\\Human\\male_v7\\export\\bhm_skeleton_v7";
const FEMALE_SKELETON: &str =
    "Data\\Objects\\Characters\\Human\\female_v2\\export\\bhf_skeleton_v2";
pub(crate) const MALE_BODY: &str = "Data\\Objects\\Characters\\Human\\male_v7\\body\\m_body";
pub(crate) const FEMALE_BODY: &str = "Data\\Objects\\Characters\\Human\\female_v2\\body\\f_body";
pub(crate) const MALE_ARMOUR: &str = "Objects/Characters/Human/male_v7/armor/test";
pub(crate) const FEMALE_ARMOUR: &str = "Objects/Characters/Human/female_v2/armor/test";

fn bone(name: &str, parent: Option<u16>, z: f32) -> TestBone {
    TestBone {
        name: name.to_owned(),
        parent,
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0, z],
    }
}

/// Returns the skeleton of the synthetic male body: `pelvis`, `spine` above it and `head` above
/// that, 1 m apart.
#[must_use]
pub fn armour_male_skeleton() -> TestSkeleton {
    TestSkeleton::new(vec![
        bone("pelvis", None, 0.0),
        bone("spine", Some(0), 1.0),
        bone("head", Some(1), 1.0),
    ])
}

/// Returns the skeleton of the synthetic female body: the male bones and a `ponytail` below the
/// head.
#[must_use]
pub fn armour_female_skeleton() -> TestSkeleton {
    let mut bones = armour_male_skeleton().bones().to_vec();
    bones.push(bone("ponytail", Some(2), 0.5));
    TestSkeleton::new(bones)
}

/// Returns the cube as a skin of `skeleton` with every vertex weighted to the bone `bone` alone.
pub(crate) fn skin(skeleton: &TestSkeleton, bone: u16) -> TestModel {
    TestModel::cube()
        .with_positions(TestPositions::Float)
        .with_nodes(Vec::new())
        .with_skeleton(skeleton)
        .with_bone_map(false, vec![vec![(bone, 255)]; 24])
}

pub(crate) fn skin_entries(path: &str, model: &TestModel) -> [TestEntry; 2] {
    [
        TestEntry::stored(format!("{path}.skin"), model.cgf()),
        TestEntry::stored(format!("{path}.skinm"), model.cgfm()),
    ]
}

fn skeleton_entry(path: &str, skeleton: &TestSkeleton) -> TestEntry {
    let model = TestModel::cube().helpers_only().with_skeleton(skeleton);
    TestEntry::stored(format!("{path}.chr"), model.cgf())
}

fn database() -> DataForgeBuilder {
    let suit = TestEntity::new(0x1, "TEST_Suit", "entities/scitem/test_suit.xml")
        .geometry("Objects/Test/crate.cgf", None)
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_suit.skin"), None)
        .geometry_variant("Female", format!("{FEMALE_ARMOUR}/f_suit.skin"), None)
        .geometry_variant("heldEntity", "Objects/Test/held.cgf", None);
    let boots = TestEntity::new(0x2, "TEST_Boots", "entities/scitem/test_boots.xml")
        .geometry("Objects/Test/crate.cgf", None)
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_boots.skin"), None);
    let cape = TestEntity::new(0x3, "TEST_Cape", "entities/scitem/test_cape.xml");
    let hat = TestEntity::new(0x4, "TEST_Hat", "entities/scitem/test_hat.xml")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_hat.skin"), None)
        .geometry_variant("Female", format!("{FEMALE_ARMOUR}/f_hat.skin"), None);
    let plain = TestEntity::new(0x5, "TEST_Plain", "entities/scitem/test_plain.xml")
        .geometry(format!("{MALE_ARMOUR}/plain.skin"), None);
    [suit, boots, cape, hat, plain]
        .iter()
        .chain(&loadout_entities())
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
}

/// Returns the Entries of a synthetic Archive that holds the skeletons and default bodies of both
/// body variants and Entities to wear on them.
///
/// The male skeleton is [`armour_male_skeleton`], the female [`armour_female_skeleton`]; each
/// body is a cube skinned to `pelvis`. The Entities, every skin a cube:
///
/// - `TEST_Suit` has a skin for `Male` and one for `Female`, weighted to `spine`, and a skin for
///   the `heldEntity` that is never worn; its main geometry is a crate the Archive lacks;
/// - `TEST_Boots` has a skin for `Male` only, so the female body cannot wear them;
/// - `TEST_Cape` names no geometry;
/// - `TEST_Hat` has a skin for `Male` and names one for `Female` that the Archive lacks;
/// - `TEST_Plain` names one skin as its main geometry, in the male folder, weighted to a bone
///   `cloth` that no skeleton has, below `spine`.
///
/// Characters with a Loadout of worn Entities, which have an `AttachDef.Type`:
///
/// - `TEST_Torso` (`Char_Armor_Torso`) is worn like `TEST_Suit`, `TEST_Helmet`
///   (`Char_Armor_Helmet`) like `TEST_Hat` on the male body only, `TEST_Plate`
///   (`Char_Armor_Torso`) takes the place of the torso, and `TEST_Pants` (`Char_Clothing_Legs`) has
///   a male skin; `TEST_Pistol` (`WeaponPersonal`) names a skin that is never worn;
/// - `TEST_Soldier` holds the torso, the pistol, the helmet, `TEST_Gone`, which no Entity has,
///   and the pants in the nested Loadout of an empty port;
/// - `TEST_Grunt` names the XML Loadout [`ARMOUR_XML_LOADOUT`], which holds the torso and the
///   helmet.
#[must_use]
pub fn armour_entries() -> Vec<TestEntry> {
    let mut entries = vec![TestEntry::zstd("Data\\Game2.dcb", database().build())];
    entries.extend(armour_files());
    entries
}

/// Returns the database of [`armour_entries`], for fixtures that add Records to it.
pub(crate) fn armour_database() -> DataForgeBuilder {
    database()
}

/// Returns the Entries of [`armour_entries`] without the database.
pub(crate) fn armour_files() -> Vec<TestEntry> {
    let male = armour_male_skeleton();
    let female = armour_female_skeleton();
    let mut cloth_bones = male.bones().to_vec();
    cloth_bones.push(bone("cloth", Some(1), 0.5));
    let cloth = TestSkeleton::new(cloth_bones);
    let library = TestMtl::multi(&[
        TestMtlMaterial::new("unused").diffuse([0.5; 3]),
        TestMtlMaterial::new("caps").diffuse([0.5; 3]),
        TestMtlMaterial::new("sides").diffuse([0.5; 3]),
    ]);
    let mut entries = vec![
        skeleton_entry(MALE_SKELETON, &male),
        skeleton_entry(FEMALE_SKELETON, &female),
        TestEntry::stored("Data\\objects\\test\\cube.mtl", library.cryxml()),
    ];
    let folder = |path: &str| format!("Data\\{}", path.replace('/', "\\"));
    entries.extend(loadout_files());
    entries.extend(skin_entries(MALE_BODY, &skin(&male, 0)));
    entries.extend(skin_entries(FEMALE_BODY, &skin(&female, 0)));
    for (path, model) in [
        (format!("{MALE_ARMOUR}/m_suit"), skin(&male, 1)),
        (format!("{FEMALE_ARMOUR}/f_suit"), skin(&female, 1)),
        (format!("{MALE_ARMOUR}/m_boots"), skin(&male, 0)),
        (format!("{MALE_ARMOUR}/m_hat"), skin(&male, 2)),
        (format!("{MALE_ARMOUR}/plain"), skin(&cloth, 3)),
    ] {
        entries.extend(skin_entries(&folder(&path), &model));
    }
    entries
}
