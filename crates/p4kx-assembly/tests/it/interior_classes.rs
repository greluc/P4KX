// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Props whose geometry comes from the class Record, the tint palettes and the material libraries
//! of an Interior built with the DataForge.

use p4kx_assembly::{
    Interior, InteriorForge, InteriorOptions, InteriorPart, InteriorPartKind, InteriorWatch,
    IssueKind,
};
use p4kx_extraction::{Archive, DataForgeEntry, select_entries};
use p4kx_gltf::GlbOptions;
use p4kx_model::srgb8_to_linear;
use p4kx_testkit::{
    CLASS_DEFAULT_TINTS, CLASS_DOOR_TINTS, CLASS_OWN_TINTS, CLASS_ROOM, check_glb,
    class_room_entries,
};

use crate::assembly_support::{TestResult, fixture};

fn build(with_forge: bool) -> Result<Interior, Box<dyn std::error::Error>> {
    let fixture = fixture(class_room_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let found = select_entries(&archive, &[CLASS_ROOM])?;
    let entry = found.first().ok_or("no container")?;
    let root = archive.open_container(entry)?;
    let forge_entry = DataForgeEntry::read(&archive)?;
    let forge = forge_entry.open()?;
    let classes = InteriorForge::new(&forge);
    Ok(Interior::build_with(
        &[&archive, &root],
        CLASS_ROOM,
        &InteriorOptions::default(),
        &InteriorWatch::none(),
        with_forge.then_some(&classes),
    )?)
}

fn named<'a>(interior: &'a Interior, name: &str) -> Result<&'a InteriorPart, String> {
    interior
        .parts()
        .iter()
        .find(|part| part.name() == name)
        .ok_or_else(|| format!("no part {name}"))
}

fn palette_of<'a>(interior: &'a Interior, name: &str) -> Result<Option<&'a str>, String> {
    Ok(named(interior, name)?.palette().map(|p| p.name.as_str()))
}

fn linear(bytes: [u8; 3]) -> [f32; 3] {
    bytes.map(srgb8_to_linear)
}

#[test]
fn entities_that_name_a_class_record_are_placed_with_the_geometry_of_the_record() -> TestResult {
    let interior = build(true)?;

    for (name, x) in [("door", 1.0), ("cabinet_a", 2.0), ("cabinet_b", 3.0)] {
        let part = named(&interior, name)?;
        assert_eq!(part.kind(), InteriorPartKind::Prop, "{name}");
        assert_eq!(part.model_path(), Some("Objects/Test/cube.cgf"));
        assert!(part.model().is_some(), "{name}");
        assert!((part.world().translation()[0] - x).abs() < 1e-4, "{name}");
    }
    assert!(
        named(&interior, "marker").is_err(),
        "a class without geometry"
    );
    assert!(
        named(&interior, "stranger").is_err(),
        "a class the forge lacks"
    );
    assert!(named(&interior, "bad_door").is_err());
    assert_eq!(
        interior.props(),
        6,
        "door, door_own, two cabinets, lamp_fixture and own"
    );
    assert_eq!(interior.modelled_parts(), interior.model_parts());
    let left_out = interior
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::EntitiesLeftOut { count: 1 }))
        .count();
    assert_eq!(left_out, 1, "the door with the bad position");
    Ok(())
}

#[test]
fn without_the_forge_class_props_and_palettes_are_not_placed() -> TestResult {
    let interior = build(false)?;

    assert_eq!(
        interior.props(),
        1,
        "only the prop with geometry of its own"
    );
    assert!(named(&interior, "door").is_err());
    assert!(interior.parts().iter().all(|part| part.palette().is_none()));
    assert!(
        interior
            .issues()
            .iter()
            .all(|issue| !matches!(issue.kind(), IssueKind::Palette { .. }))
    );
    Ok(())
}

#[test]
fn the_palette_of_a_part_is_the_one_it_names_else_the_default_of_its_library() -> TestResult {
    let interior = build(true)?;

    assert_eq!(palette_of(&interior, "door")?, Some("p_door"));
    assert_eq!(
        palette_of(&interior, "door_own")?,
        Some("p_own"),
        "the palette the entity chooses replaces that of its class Record"
    );
    let default_of = |name: &str| -> Result<Option<String>, String> {
        Ok(named(&interior, name)?
            .materials()
            .library()
            .and_then(|library| library.default_palette.clone()))
    };
    assert_eq!(default_of("door")?, None);
    assert!(
        default_of("door_own")?.is_some(),
        "the library the entity chooses replaces that of its class Record"
    );
    assert_eq!(
        palette_of(&interior, "own")?,
        Some("p_own"),
        "an entity's palette"
    );
    assert_eq!(
        palette_of(&interior, "cabinet_a")?,
        Some("p_default"),
        "the library's default"
    );
    let (first, second) = (
        named(&interior, "cabinet_a")?.palette(),
        named(&interior, "cabinet_b")?.palette(),
    );
    assert!(
        first.zip(second).is_some_and(|(a, b)| std::ptr::eq(a, b)),
        "one palette, read once"
    );
    assert_eq!(palette_of(&interior, "lamp_fixture")?, None);
    Ok(())
}

#[test]
fn the_palettes_of_the_object_table_are_applied_and_a_missing_one_is_listed() -> TestResult {
    let interior = build(true)?;

    let cubes: Vec<_> = interior
        .parts()
        .iter()
        .filter(|part| part.kind() == InteriorPartKind::Model)
        .collect();
    let [room, missing] = cubes.as_slice() else {
        return Err("two placed cubes".into());
    };
    assert_eq!(room.palette().map(|p| p.name.as_str()), Some("p_room"));
    assert!(missing.palette().is_none());
    let reasons: Vec<String> = interior
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::Palette { .. }))
        .map(|issue| issue.kind().to_string())
        .collect();
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(reasons[0].contains("p_missing"), "{reasons:?}");
    Ok(())
}

#[test]
fn the_material_library_a_prop_names_replaces_the_one_of_its_model() -> TestResult {
    let interior = build(true)?;

    let library = |name: &str| -> Result<Option<usize>, String> {
        Ok(named(&interior, name)?
            .materials()
            .library()
            .map(|library| library.materials.len()))
    };
    assert_eq!(library("door")?, Some(3), "painted: unused, caps, sides");
    assert_eq!(library("own")?, Some(3));
    assert_eq!(library("cabinet_a")?, Some(3));
    let fixture = named(&interior, "lamp_fixture")?;
    let door = named(&interior, "door")?;
    assert!(
        !std::ptr::eq(fixture.materials(), door.materials()),
        "the fixture keeps the library of its model"
    );
    Ok(())
}

#[test]
fn the_class_entity_keeps_its_baked_in_lights() -> TestResult {
    let interior = build(true)?;

    let light = named(&interior, "Light-A")?;
    assert_eq!(light.kind(), InteriorPartKind::Light);
    assert!(
        (light.world().translation()[1] - 6.0).abs() < 1e-4
            && (light.world().translation()[2] - 1.0).abs() < 1e-4,
        "{:?}",
        light.world()
    );
    assert_eq!(
        light.light().and_then(|l| l.group.as_deref()),
        Some("lamp_fixture")
    );
    Ok(())
}

#[test]
fn the_glb_draws_each_part_with_its_palette() -> TestResult {
    let interior = build(true)?;
    let mut bytes = Vec::new();

    interior.write_glb(&GlbOptions::default(), &mut bytes)?;

    let glb = check_glb(&bytes)?;
    let colours: Vec<[f32; 3]> = glb
        .document
        .materials()
        .map(|material| {
            let [r, g, b, _] = material.pbr_metallic_roughness().base_color_factor();
            [r, g, b]
        })
        .collect();
    for (what, tints) in [
        ("door", CLASS_DOOR_TINTS),
        ("own", CLASS_OWN_TINTS),
        ("default", CLASS_DEFAULT_TINTS),
    ] {
        let wanted = linear(tints[0]);
        assert!(
            colours
                .iter()
                .any(|c| c.iter().zip(wanted).all(|(a, w)| (a - w).abs() < 1e-4)),
            "{what}: {wanted:?} in {colours:?}"
        );
    }
    Ok(())
}
