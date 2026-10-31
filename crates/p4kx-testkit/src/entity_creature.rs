// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    DataForgeBuilder, TestBone, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestModel,
    TestPort, TestSkeleton, entity_schema,
};

/// A creature whose root geometry is a skeleton file (`.chr`) with the bones `pelvis`, `root`
/// and `tip`, wearing a skin of the bones `root` and `tip` on a skin port.
pub const CREATURE: &str = "Test_Creature";

/// Like [`CREATURE`], but its skeleton lacks the bone `tip` that its skin is weighted to.
pub const CREATURE_SHORT_SKELETON: &str = "Test_Creature_Short";

/// A creature whose root geometry is a character definition that names the skeleton file of
/// [`CREATURE`], wearing the same skin.
pub const CREATURE_DEFINITION: &str = "Test_Creature_Definition";

/// Like [`CREATURE_DEFINITION`], but the skeleton file it names is not in the Archive.
pub const CREATURE_NO_SKELETON: &str = "Test_Creature_NoSkeleton";

const FOLDER: &str = "Objects/Characters/Creatures/Test";

fn definition(skeleton: &str) -> String {
    format!(
        r#"<CharacterDefinition>
  <Model File="{FOLDER}/{skeleton}"/>
  <AttachmentList>
    <Attachment Type="CA_SKIN" AName="Body" Binding="{FOLDER}/creature.skin"/>
  </AttachmentList>
</CharacterDefinition>"#
    )
}

fn bone(name: &str, parent: Option<u16>) -> TestBone {
    TestBone {
        name: name.to_owned(),
        parent,
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0, 1.0],
    }
}

fn skeleton_file(bones: Vec<TestBone>) -> Vec<u8> {
    TestModel::cube()
        .helpers_only()
        .with_skeleton(&TestSkeleton::new(bones))
        .cgf()
}

fn database() -> DataForgeBuilder {
    let creature = |id, class: &str, geometry: &str| {
        TestEntity::new(id, class, format!("entities/{class}.xml"))
            .geometry(geometry, None)
            .port(TestPort::new("Body_ItemPort").skin())
            .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "Body_ItemPort",
                "Test_Creature_Skin",
            )]))
    };
    let skin = TestEntity::new(
        0x10,
        "Test_Creature_Skin",
        "entities/Test_Creature_Skin.xml",
    )
    .geometry(format!("{FOLDER}/creature.skin"), None);
    [
        creature(0x1, CREATURE, &format!("{FOLDER}/creature_skel.chr")),
        creature(
            0x2,
            CREATURE_SHORT_SKELETON,
            &format!("{FOLDER}/short_skel.chr"),
        ),
        creature(
            0x3,
            CREATURE_DEFINITION,
            &format!("{FOLDER}/definition.cdf"),
        ),
        creature(
            0x4,
            CREATURE_NO_SKELETON,
            &format!("{FOLDER}/no_skeleton.cdf"),
        ),
        skin,
    ]
    .iter()
    .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
        builder.record(entity.record())
    })
}

/// Returns the Entries of a synthetic Archive that holds creatures with a skeleton and a skin.
///
/// The four creatures [`CREATURE`], [`CREATURE_SHORT_SKELETON`], [`CREATURE_DEFINITION`] and
/// [`CREATURE_NO_SKELETON`] each hold the same two-bone skin on the skin port `Body_ItemPort`.
#[must_use]
pub fn creature_entries() -> Vec<TestEntry> {
    let skin = TestModel::two_bone_skin();
    let entry = |name: &str, bytes: Vec<u8>| {
        TestEntry::stored(
            format!("Data\\{}", format!("{FOLDER}/{name}").replace('/', "\\")),
            bytes,
        )
    };
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database().build()),
        entry(
            "creature_skel.chr",
            skeleton_file(vec![
                bone("pelvis", None),
                bone("root", Some(0)),
                bone("tip", Some(1)),
            ]),
        ),
        entry(
            "short_skel.chr",
            skeleton_file(vec![bone("pelvis", None), bone("root", Some(0))]),
        ),
        entry("creature.skin", skin.cgf()),
        entry("creature.skinm", skin.cgfm()),
        entry(
            "definition.cdf",
            definition("creature_skel.chr").into_bytes(),
        ),
        entry(
            "no_skeleton.cdf",
            definition("absent_skel.chr").into_bytes(),
        ),
    ]
}
