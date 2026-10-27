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

pub(crate) fn zone_shirt(chunks: &[TestZoneChunk]) -> TestEntity {
    TestEntity::new(0x30, ZONE_SHIRT, "entities/scitem/test_zone_shirt.xml")
        .attach_type("Char_Clothing_Torso_0")
        .geometry_variant("Male", format!("{MALE_ARMOUR}/m_suit.skin"), None)
        .geometry_variant("Female", format!("{FEMALE_ARMOUR}/f_suit.skin"), None)
        .component(clothing_component(chunks))
}

/// Returns the Entries of an Archive of [`armour_entries`](crate::armour_entries) with zoned
/// bodies.
///
/// The male and the female body skin are [`TestModel::zoned`] by the places `bodies` give.
/// [`ZONE_SHIRT`] (`Char_Clothing_Torso_0`) covers `chunks` and is worn like `TEST_Torso`. The
/// material of the zones is layered and has no diffuse map.
#[must_use]
pub fn zone_entries(bodies: [&[[f32; 2]]; 2], chunks: &[TestZoneChunk]) -> Vec<TestEntry> {
    let database = armour_database()
        .record(zone_shirt(chunks).record())
        .build();
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
    for (path, skeleton, places) in [
        (MALE_BODY, &male, bodies[0]),
        (FEMALE_BODY, &female, bodies[1]),
    ] {
        let model = TestModel::zoned(places)
            .with_skeleton(skeleton)
            .with_bone_map(false, vec![vec![(0, 255)]; places.len() * 3]);
        entries.extend(skin_entries(path, &model));
    }
    entries
}

/// Returns the Entries of [`zone_entries`] with a diffuse map in the layer of the zones' material.
///
/// The layer names the file `Objects/Test/zone_layer.mtl`, whose `TexSlot1` is a diffuse map.
#[must_use]
pub fn zone_entries_with_layer_textures(
    bodies: [&[[f32; 2]]; 2],
    chunks: &[TestZoneChunk],
) -> Vec<TestEntry> {
    let mut entries = zone_entries(bodies, chunks);
    let library = format!("{LIBRARY}.mtl");
    entries.retain(|entry| !entry.path.eq_ignore_ascii_case(&library));
    let layered = TestMtlMaterial::new("caps")
        .diffuse([0.5; 3])
        .layer_with(&[("Name", "Primary"), ("Path", "Objects/Test/zone_layer.mtl")]);
    entries.push(TestEntry::stored(
        library,
        TestMtl::multi(&[
            TestMtlMaterial::new("unused"),
            layered,
            TestMtlMaterial::new("sides").diffuse([0.5; 3]),
        ])
        .cryxml(),
    ));
    let layer = TestMtlMaterial::new("layer")
        .shader("Layer")
        .diffuse([1.0; 3])
        .texture("TexSlot1", "Objects/Test/zone_layer_diff.tif");
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\zone_layer.mtl",
        TestMtl::single(&layer).cryxml(),
    ));
    entries
}
