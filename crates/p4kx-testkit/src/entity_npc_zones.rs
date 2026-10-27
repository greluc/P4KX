// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_armour::{MALE_ARMOUR, armour_database};
use crate::entity_clothing::TestZoneChunk;
use crate::entity_zones::{zone_entries, zone_shirt};
use crate::{TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, ZONE_SHIRT};

/// The class name of the character of [`npc_zone_entries`].
pub const NPC_ZONE_CLASS: &str = "TEST_ZoneNpc";

const MALE_SKIN: &str = "Objects/Characters/Human/male_v7/body/m_body.skin";
const FEMALE_SKIN: &str = "Objects/Characters/Human/female_v2/body/f_body.skin";

/// How [`npc_zone_entries`] builds the character.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NpcZoneOptions {
    /// Makes the character female: its head is `Female` and its body the female skin.
    pub female: bool,
    /// Leaves the head out; the shirt then names the male suit skin as its main geometry, so that
    /// it has a model without a body to choose by.
    pub headless: bool,
}

/// Returns the Entries of [`zone_entries`] with a character whose body Entity names the zoned
/// body skin and holds [`ZONE_SHIRT`], which covers `chunks`.
///
/// [`NPC_ZONE_CLASS`] has the Loadout `Body_ItemPort` holding `TEST_ZoneNpcBody` (`Char_Body`,
/// geometry the body skin of `options`), whose Loadout puts a head (`Char_Head`, `Male` or
/// `Female`) on `Head_ItemPort` unless `options.headless`, and the shirt on `Clothing_Torso_0`.
#[must_use]
pub fn npc_zone_entries(
    bodies: [&[[f32; 2]]; 2],
    chunks: &[TestZoneChunk],
    options: NpcZoneOptions,
) -> Vec<TestEntry> {
    let head = TestEntity::new(
        0x31,
        "TEST_ZoneNpcHead",
        "entities/scitem/test_zone_head.xml",
    )
    .attach_type("Char_Head")
    .sub_type(if options.female { "Female" } else { "Male" });
    let body = TestEntity::new(
        0x32,
        "TEST_ZoneNpcBody",
        "entities/scitem/test_zone_body.xml",
    )
    .attach_type("Char_Body")
    .geometry(
        if options.female {
            FEMALE_SKIN
        } else {
            MALE_SKIN
        },
        None::<&str>,
    );
    let mut worn = vec![TestLoadoutEntry::class("Clothing_Torso_0", ZONE_SHIRT)];
    let mut shirt = zone_shirt(chunks);
    if options.headless {
        shirt = shirt.geometry(format!("{MALE_ARMOUR}/m_suit.skin"), None::<&str>);
    } else {
        worn.insert(
            0,
            TestLoadoutEntry::class("Head_ItemPort", "TEST_ZoneNpcHead"),
        );
    }
    let npc = TestEntity::new(0x33, NPC_ZONE_CLASS, "entities/scitem/test_zone_npc.xml").loadout(
        TestLoadout::Manual(vec![
            TestLoadoutEntry::class("Body_ItemPort", "TEST_ZoneNpcBody")
                .loadout(TestLoadout::Manual(worn)),
        ]),
    );
    let database = [shirt, head, body, npc]
        .iter()
        .fold(armour_database(), |builder, entity| {
            builder.record(entity.record())
        })
        .build();
    let mut entries = zone_entries(bodies, chunks);
    entries.retain(|entry| !entry.path.eq_ignore_ascii_case("Data\\Game2.dcb"));
    entries.push(TestEntry::zstd("Data\\Game2.dcb", database));
    entries
}
