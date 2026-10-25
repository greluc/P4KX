// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_armour::{
    FEMALE_ARMOUR, FEMALE_BODY, MALE_ARMOUR, MALE_BODY, armour_database, armour_female_skeleton,
    armour_files, armour_male_skeleton, skin_entries,
};
use crate::entity_clothing::{TestZoneChunk, clothing_component};
use crate::{TestEntity, TestEntry, TestModel, TestMtl, TestMtlMaterial};

const LIBRARY: &str = "Data\\objects\\test\\cube";

/// The class name of the shirt of [`zone_entries`].
pub const ZONE_SHIRT: &str = "TEST_ZoneShirt";

/// Returns the Entries of an Archive of [`armour_entries`](crate::armour_entries) with zoned bodies.
///
/// The male body skin has `zones` triangles and the female one `zones + 1`, each its own submesh
/// numbered by its position. [`ZONE_SHIRT`] (`Char_Clothing_Torso_0`) covers `chunks` and is worn like `TEST_Torso`. The
/// material of the zones is layered and has no diffuse map.
#[must_use]
pub fn zone_entries(zones: usize, chunks: &[TestZoneChunk]) -> Vec<TestEntry> {
    let shirt = TestEntity::new(0x30, ZONE_SHIRT, "entities/scitem/test_zone_shirt.xml")
        .attach_type("Char_Clothing_Torso_0")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_suit.skin"), None)
        .geometry_variant("Female", format!("{FEMALE_ARMOUR}/f_suit.skin"), None)
        .component(clothing_component(chunks));
    let database = armour_database().record(shirt.record()).build();
    let mut entries = vec![TestEntry::zstd("Data\\Game2.dcb", database)];
    let replaced = [MALE_BODY, FEMALE_BODY, LIBRARY].map(str::to_ascii_lowercase);
    entries.extend(armour_files().into_iter().filter(|entry| {
        let path = entry.path.to_ascii_lowercase();
        !replaced.iter().any(|name| path.starts_with(name))
    }));
    let layered = TestMtlMaterial::new("caps")
        .diffuse([0.5; 3])
        .layer("Primary", "0");
    entries.push(TestEntry::stored(
        format!("{LIBRARY}.mtl"),
        TestMtl::multi(&[
            TestMtlMaterial::new("unused"),
            layered,
            TestMtlMaterial::new("sides").diffuse([0.5; 3]),
        ])
        .cryxml(),
    ));
    let male = armour_male_skeleton();
    let female = armour_female_skeleton();
    for (path, skeleton, count) in [(MALE_BODY, &male, zones), (FEMALE_BODY, &female, zones + 1)] {
        let model = TestModel::zoned(count)
            .with_skeleton(skeleton)
            .with_bone_map(false, vec![vec![(0, 255)]; count * 3]);
        entries.extend(skin_entries(path, &model));
    }
    entries
}
