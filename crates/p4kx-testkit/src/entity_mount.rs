// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_armour::{MALE_ARMOUR, armour_database, armour_files};
use crate::{
    DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestModel, TestNode,
    TestPort,
};

/// The class name of the character whose backpack hangs on a bone its armour has.
pub const MOUNT_COURIER: &str = "Mount_Courier";

/// The class name of the character whose backpack port names a bone its armour lacks.
pub const MOUNT_DRIFTER: &str = "Mount_Drifter";

/// The bone below which the satchel of [`MOUNT_COURIER`] hangs: the nearest bone of the body
/// skeleton above the bone `cloth` of its armour.
pub const MOUNT_BONE: &str = "spine";

/// Where the helper of the satchel sits in its model.
pub const SATCHEL_HELPER_Z: f32 = 0.5;

const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

fn satchel_model() -> TestModel {
    let mut helper = IDENTITY;
    helper[2][3] = SATCHEL_HELPER_Z;
    TestModel::cube().with_nodes(vec![
        TestNode {
            name: "satchel".to_owned(),
            parent: None,
            local: IDENTITY,
            kind: 0,
        },
        TestNode {
            name: "satchel_loc".to_owned(),
            parent: Some(0),
            local: helper,
            kind: 3,
        },
    ])
}

fn carrier(guid: u128, class: &str, helper: &str) -> TestEntity {
    TestEntity::new(guid, class, format!("entities/scitem/{class}.xml"))
        .attach_type("Char_Armor_Torso")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/plain.skin"), None)
        .port(
            TestPort::new("pack")
                .item_type("Char_Armor_Backpack", Vec::<String>::new())
                .helper(helper, [0.0, 0.1, 0.2], [0.0; 3])
                .item_helper("satchel_loc"),
        )
        .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
            "pack",
            "Mount_Satchel",
        )]))
}

fn character(guid: u128, class: &str, torso: &str) -> TestEntity {
    TestEntity::new(guid, class, format!("entities/scitem/{class}.xml")).loadout(
        TestLoadout::Manual(vec![TestLoadoutEntry::class("torso", torso)]),
    )
}

fn database() -> DataForgeBuilder {
    let satchel = TestEntity::new(0x40, "Mount_Satchel", "entities/scitem/mount_satchel.xml")
        .attach_type("Char_Armor_Backpack")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/satchel.cgf"), None);
    [
        satchel,
        carrier(0x41, "Mount_Carrier", "cloth"),
        carrier(0x42, "Mount_Lost", "nowhere"),
        character(0x43, MOUNT_COURIER, "Mount_Carrier"),
        character(0x44, MOUNT_DRIFTER, "Mount_Lost"),
    ]
    .iter()
    .fold(armour_database(), |builder, entity| {
        builder.record(entity.record())
    })
}

/// Returns the Entries of a synthetic Archive that holds the bodies of [`armour_entries`] and
/// characters that wear a backpack at a port of their armour.
///
/// `Mount_Satchel` (`Char_Armor_Backpack`) is a model without bone weights, with the helper
/// `satchel_loc` at (0, 0, [`SATCHEL_HELPER_Z`]). `Mount_Carrier` (`Char_Armor_Torso`) is worn
/// like `TEST_Plain`, which has the bone `cloth` below `spine`, and has the port `pack`, a bone
/// attachment to `cloth` offset by (0, 0.1, 0.2) that holds the satchel. [`MOUNT_COURIER`] wears
/// the carrier; `Mount_Lost` is a carrier whose port names the bone `nowhere`, worn by
/// [`MOUNT_DRIFTER`].
///
/// [`armour_entries`]: crate::armour_entries
#[must_use]
pub fn mounted_item_entries() -> Vec<TestEntry> {
    let satchel = satchel_model();
    let mut entries = vec![TestEntry::zstd("Data\\Game2.dcb", database().build())];
    entries.extend(armour_files());
    entries.push(TestEntry::stored(
        format!("Data\\{}\\satchel.cgf", MALE_ARMOUR.replace('/', "\\")),
        satchel.cgf(),
    ));
    entries.push(TestEntry::stored(
        format!("Data\\{}\\satchel.cgfm", MALE_ARMOUR.replace('/', "\\")),
        satchel.cgfm(),
    ));
    entries
}
