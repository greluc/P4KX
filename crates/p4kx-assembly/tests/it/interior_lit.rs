// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The props and lights of the synthetic lit room: placement, glTF lights and what is reported.

use p4kx_assembly::{Interior, InteriorOptions, InteriorPart, InteriorPartKind, IssueKind};
use p4kx_extraction::{Archive, select_entries};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::gltf::khr_lights_punctual::Kind;
use p4kx_testkit::{LIT_ROOM, check_glb, lit_room_entries};

use crate::assembly_support::{TestResult, fixture};

fn lit_room() -> Result<Interior, Box<dyn std::error::Error>> {
    let fixture = fixture(lit_room_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let found = select_entries(&archive, &[LIT_ROOM])?;
    let entry = found.first().ok_or("no container")?;
    let root = archive.open_container(entry)?;
    Ok(Interior::build(
        &[&archive, &root],
        LIT_ROOM,
        &InteriorOptions::default(),
    )?)
}

fn named<'a>(interior: &'a Interior, name: &str) -> Result<&'a InteriorPart, String> {
    interior
        .parts()
        .iter()
        .find(|part| part.name() == name)
        .ok_or_else(|| format!("no part {name}"))
}

fn close(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 1e-4)
}

#[test]
fn props_are_placed_like_other_models_and_missing_ones_are_reported() -> TestResult {
    let interior = lit_room()?;

    assert_eq!(interior.props(), 2);
    assert_eq!(interior.model_parts(), 2);
    assert_eq!(interior.modelled_parts(), 1, "the ghost has no model");
    let stool = named(&interior, "stool")?;
    assert_eq!(stool.kind(), InteriorPartKind::Prop);
    assert_eq!(stool.model_path(), Some("objects/test/cube.cgf"));
    assert!(stool.model().is_some());
    assert!(close(&stool.world().translation(), &[2.0, 0.0, 0.0]));
    let ghost = named(&interior, "ghost")?;
    assert!(ghost.model().is_none());
    let missing: Vec<_> = interior
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::GeometryMissing { .. }))
        .collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].name(), "ghost");
    Ok(())
}

#[test]
fn lights_gltf_can_express_become_parts_with_their_parameters() -> TestResult {
    let interior = lit_room()?;

    assert_eq!(interior.lights(), 5, "lamp, beam, sun and the group's two");
    let lamp = named(&interior, "lamp")?;
    assert_eq!(lamp.kind(), InteriorPartKind::Light);
    let light = lamp.light().ok_or("no light")?;
    assert_eq!(light.light_type, "Omni");
    assert!((light.intensity - 3.0).abs() < 1e-6);
    let group_light = named(&interior, "Light-A")?;
    assert!(
        close(&group_light.world().translation(), &[1.0, -4.0, 2.0]),
        "{:?}",
        group_light.world()
    );
    Ok(())
}

#[test]
fn light_types_gltf_cannot_express_and_unusable_entities_are_reported() -> TestResult {
    let interior = lit_room()?;

    let mut unsupported: Vec<(String, usize)> = interior
        .issues()
        .iter()
        .filter_map(|issue| match issue.kind() {
            IssueKind::LightUnsupported { light_type, count } => Some((light_type.clone(), *count)),
            _ => None,
        })
        .collect();
    unsupported.sort();
    assert_eq!(
        unsupported,
        [("Ambient".to_owned(), 1), ("Planar".to_owned(), 2)]
    );
    let left_out = interior
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::EntitiesLeftOut { count: 1 }))
        .count();
    assert_eq!(left_out, 1, "the prop without a usable position");
    Ok(())
}

#[test]
fn the_glb_writes_the_lights_with_the_lights_extension() -> TestResult {
    let interior = lit_room()?;
    let mut bytes = Vec::new();

    let summary = interior.write_glb(&GlbOptions::default(), &mut bytes)?;

    let glb = check_glb(&bytes)?;
    assert_eq!(summary.lights, 5);
    let light_of = |name: &str| {
        glb.document
            .nodes()
            .find(|node| node.name() == Some(name))
            .and_then(|node| node.light())
            .ok_or_else(|| format!("no light at {name}"))
    };
    let lamp = light_of("lamp")?;
    assert!(matches!(lamp.kind(), Kind::Point));
    let [red, green, blue] = lamp.color();
    assert!((red - 1.0).abs() < 1e-5, "sRGB 1 stays 1 in linear");
    assert!((green - 0.2140).abs() < 1e-3, "sRGB 0.5 is 0.214 linear");
    assert!((blue - 0.0509).abs() < 1e-3, "sRGB 0.25 is 0.051 linear");
    assert!((lamp.intensity() - 3.0).abs() < 1e-6);
    assert_eq!(lamp.range(), Some(6.0));
    let beam = light_of("beam")?;
    assert!(matches!(
        beam.kind(),
        Kind::Spot { inner_cone_angle, outer_cone_angle }
            if inner_cone_angle.abs() < 1e-6
                && (outer_cone_angle - 30.0_f32.to_radians()).abs() < 1e-5
    ));
    assert!(matches!(light_of("sun")?.kind(), Kind::Directional));
    assert_eq!(
        light_of("Light-B")?.range(),
        None,
        "a radius of 0 is no range"
    );
    Ok(())
}
