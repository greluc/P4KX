// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_armour::armour_database;
use crate::entity_clothing::TestZoneChunk;
use crate::entity_zones::{zone_entries, zone_shirt};
use crate::{TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, ZONE_SHIRT};

/// The class name of the character of [`npc_zone_entries`].
pub const NPC_ZONE_CLASS: &str = "TEST_ZoneNpc";

const BODY_SKIN: &str = "Objects/Characters/Human/male_v7/body/m_body.skin";

/// Returns the Entries of [`zone_entries`] with a male character whose body Entity names the
/// zoned body skin and holds [`ZONE_SHIRT`], which covers `chunks`.
///
/// [`NPC_ZONE_CLASS`] has the Loadout `Body_ItemPort` holding `TEST_ZoneNpcBody` (`Char_Body`,
/// geometry the male body skin), whose Loadout puts a head (`Char_Head`, `Male`) on
/// `Head_ItemPort` and the shirt on `Clothing_Torso_0`.
#[must_use]
pub fn npc_zone_entries(bodies: [&[[f32; 2]]; 2], chunks: &[TestZoneChunk]) -> Vec<TestEntry> {
    let head = TestEntity::new(
        0x31,
        "TEST_ZoneNpcHead",
        "entities/scitem/test_zone_head.xml",
    )
    .attach_type("Char_Head")
    .sub_type("Male");
    let body = TestEntity::new(
        0x32,
        "TEST_ZoneNpcBody",
        "entities/scitem/test_zone_body.xml",
    )
    .attach_type("Char_Body")
    .geometry(BODY_SKIN, None::<&str>);
    let npc = TestEntity::new(0x33, NPC_ZONE_CLASS, "entities/scitem/test_zone_npc.xml").loadout(
        TestLoadout::Manual(vec![
            TestLoadoutEntry::class("Body_ItemPort", "TEST_ZoneNpcBody").loadout(
                TestLoadout::Manual(vec![
                    TestLoadoutEntry::class("Head_ItemPort", "TEST_ZoneNpcHead"),
                    TestLoadoutEntry::class("Clothing_Torso_0", ZONE_SHIRT),
                ]),
            ),
        ]),
    );
    let database = [zone_shirt(chunks), head, body, npc]
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
