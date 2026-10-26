// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading the props and lights of a `.soc` file's entity chunk.

use p4kx_model::{LightState, SocEntities};
use p4kx_testkit::{TestLight, TestLightState, TestSoc, TestSocEntity};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];
const QUARTER_Z: [f64; 4] = [
    std::f64::consts::FRAC_1_SQRT_2,
    0.0,
    0.0,
    std::f64::consts::FRAC_1_SQRT_2,
];

const fn state(intensity: f64, colour: [f64; 3]) -> TestLightState {
    TestLightState {
        intensity,
        colour,
        temperature: 3000.0,
    }
}

fn read(soc: &TestSoc) -> Result<SocEntities, Box<dyn std::error::Error>> {
    SocEntities::read(&soc.bytes())?.ok_or_else(|| "no entity chunk".into())
}

#[test]
fn props_with_their_own_geometry_are_placed_with_position_rotation_and_scale() -> TestResult {
    let soc = TestSoc::new()
        .entity(TestSocEntity::prop(
            "chair",
            [1.0, 2.0, 3.0],
            QUARTER_Z,
            "objects/test/chair.cga",
        ))
        .entity(
            TestSocEntity::prop("crate", [0.0; 3], IDENTITY, "objects/test/crate.cgf")
                .scaled([2.0, 2.0, 2.0]),
        )
        .entity(TestSocEntity::plain("room", "Room_RN", [0.0; 3]));

    let entities = read(&soc)?;

    assert_eq!(entities.props.len(), 2);
    assert_eq!(entities.other, 1, "the room is neither prop nor light");
    let chair = &entities.props[0];
    assert_eq!(chair.name, "chair");
    assert_eq!(chair.model, "objects/test/chair.cga");
    let rows = chair.transform.rows;
    assert!((rows[0][1] + 1.0).abs() < 1e-5 && (rows[1][0] - 1.0).abs() < 1e-5);
    assert_eq!(chair.transform.translation(), [1.0, 2.0, 3.0]);
    assert!((entities.props[1].transform.rows[0][0] - 2.0).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_light_entity_reads_type_colour_intensity_and_range() -> TestResult {
    let light = TestLight::new("Omni")
        .radius(7.5)
        .with_temperature()
        .default_state(state(4.0, [0.5, 0.25, 1.0]));
    let soc = TestSoc::new().entity(TestSocEntity::light(
        "lamp",
        [1.0, 0.0, 2.0],
        IDENTITY,
        &light,
    ));

    let entities = read(&soc)?;

    let [lamp] = entities.lights.as_slice() else {
        return Err("one light".into());
    };
    assert_eq!(lamp.name, "lamp");
    assert_eq!(lamp.light_type, "Omni");
    assert_eq!(lamp.state, LightState::Default);
    assert_eq!(lamp.colour, [0.5, 0.25, 1.0]);
    assert!((lamp.intensity - 4.0).abs() < 1e-6);
    assert!((lamp.radius - 7.5).abs() < 1e-6);
    assert!(lamp.use_temperature);
    assert!((lamp.temperature - 3000.0).abs() < 1e-3);
    assert_eq!(lamp.transform.translation(), [1.0, 0.0, 2.0]);
    Ok(())
}

#[test]
fn lights_of_a_group_follow_the_group_and_their_relative_transform() -> TestResult {
    let soc = TestSoc::new().entity(TestSocEntity::light_group(
        "lg_hall",
        [10.0, 0.0, 0.0],
        QUARTER_Z,
        &[
            (
                "Light-1",
                TestLight::new("Projector").fov(60.0),
                IDENTITY,
                [1.0, 0.0, 0.0],
            ),
            ("Light-2", TestLight::new("SoftOmni"), IDENTITY, [0.0; 3]),
        ],
    ));

    let entities = read(&soc)?;

    assert_eq!(entities.lights.len(), 2);
    let first = &entities.lights[0];
    assert_eq!(first.name, "Light-1");
    assert_eq!(first.group.as_deref(), Some("lg_hall"));
    assert!((first.fov - 60.0).abs() < 1e-6);
    let [x, y, z] = first.transform.translation();
    assert!(
        (x - 10.0).abs() < 1e-5 && (y - 1.0).abs() < 1e-5 && z.abs() < 1e-5,
        "the group's quarter turn moves the relative +X to +Y: {x} {y} {z}"
    );
    Ok(())
}

#[test]
fn the_first_state_that_is_on_gives_the_values() -> TestResult {
    let off = TestLight::new("Omni")
        .default_state(state(0.0, [1.0; 3]))
        .state("auxiliaryState", state(0.0, [1.0; 3]))
        .state("emergencyState", state(0.8, [1.0, 0.0, 0.0]))
        .state("cinematicState", state(3.0, [0.0, 1.0, 0.0]));
    let none = TestLight::new("Omni").default_state(state(0.0, [0.1, 0.2, 0.3]));
    let soc = TestSoc::new()
        .entity(TestSocEntity::light("a", [0.0; 3], IDENTITY, &off))
        .entity(TestSocEntity::light("b", [0.0; 3], IDENTITY, &none));

    let entities = read(&soc)?;

    assert_eq!(entities.lights[0].state, LightState::Emergency);
    assert_eq!(entities.lights[0].colour, [1.0, 0.0, 0.0]);
    assert_eq!(entities.lights[1].state, LightState::Default);
    assert!(entities.lights[1].intensity.abs() < f32::EPSILON);
    Ok(())
}

#[test]
fn values_that_overflow_single_precision_are_left_out_and_counted() -> TestResult {
    let soc = TestSoc::new()
        .entity(
            TestSocEntity::prop("far", [0.0; 3], IDENTITY, "objects/test/a.cgf")
                .raw_pos("1e39,0,0"),
        )
        .entity(
            TestSocEntity::prop("huge", [0.0; 3], IDENTITY, "objects/test/b.cgf")
                .scaled([1e300, 1.0, 1.0]),
        )
        .entity(TestSocEntity::light_group(
            "lg",
            [0.0; 3],
            IDENTITY,
            &[("far", TestLight::new("Omni"), IDENTITY, [1e39, 0.0, 0.0])],
        ))
        .entity(TestSocEntity::prop(
            "good",
            [0.0; 3],
            IDENTITY,
            "objects/test/c.cgf",
        ));

    let entities = read(&soc)?;

    assert_eq!(entities.props.len(), 1);
    assert_eq!(entities.lights.len(), 0);
    assert_eq!(entities.invalid, 3);
    Ok(())
}

#[test]
fn entities_with_unusable_transforms_are_left_out_and_counted() -> TestResult {
    let soc = TestSoc::new()
        .entity(
            TestSocEntity::prop("bad", [0.0; 3], IDENTITY, "objects/test/a.cgf").raw_pos("1,x,3"),
        )
        .entity(
            TestSocEntity::prop("flat", [0.0; 3], IDENTITY, "objects/test/b.cgf")
                .scaled([0.0, 1.0, 1.0]),
        )
        .entity(TestSocEntity::prop(
            "good",
            [0.0; 3],
            IDENTITY,
            "objects/test/c.cgf",
        ));

    let entities = read(&soc)?;

    assert_eq!(entities.props.len(), 1);
    assert_eq!(entities.invalid, 2);
    Ok(())
}

#[test]
fn entities_that_name_a_class_record_and_no_geometry_are_listed_for_the_caller() -> TestResult {
    let guid = "9bf38988-1d51-4f37-996a-00139e66420b";
    let soc = TestSoc::new()
        .entity(TestSocEntity::classed(
            "door",
            "Door",
            guid,
            [1.0, 2.0, 3.0],
            IDENTITY,
        ))
        .entity(TestSocEntity::classed("bad", "Door", guid, [0.0; 3], IDENTITY).raw_pos("1,x,3"))
        .entity(TestSocEntity::classed(
            "lamp", "Light", guid, [0.0; 3], IDENTITY,
        ))
        .entity(TestSocEntity::plain("room", "Room_RN", [0.0; 3]));

    let entities = read(&soc)?;

    let [door, bad] = entities.classed.as_slice() else {
        return Err("the door and the door with a bad position".into());
    };
    assert_eq!((door.name.as_str(), door.class.as_str()), ("door", guid));
    assert_eq!(
        door.transform.ok_or("no transform")?.translation(),
        [1.0, 2.0, 3.0]
    );
    assert!(bad.transform.is_none());
    assert_eq!(
        entities.other, 2,
        "the room and the light without component"
    );
    assert_eq!(entities.invalid, 0, "the caller counts a bad class entity");
    Ok(())
}

#[test]
fn an_entity_with_a_class_record_keeps_its_baked_in_lights() -> TestResult {
    let soc = TestSoc::new().entity(
        TestSocEntity::classed(
            "lamp_fixture",
            "LightGroupPoweredItem",
            "7fe31d04-bcf0-4013-8b27-b48fef8cd5e6",
            [4.0, 0.0, 0.0],
            IDENTITY,
        )
        .with_lights(&[("Light-1", TestLight::new("Omni"), IDENTITY, [0.0, 0.0, 1.0])]),
    );

    let entities = read(&soc)?;

    assert_eq!(entities.classed.len(), 1);
    let [light] = entities.lights.as_slice() else {
        return Err("one light".into());
    };
    assert_eq!(light.group.as_deref(), Some("lamp_fixture"));
    assert_eq!(light.transform.translation(), [4.0, 0.0, 1.0]);
    Ok(())
}

#[test]
fn a_prop_with_geometry_keeps_its_baked_in_lights_and_its_material_and_palette() -> TestResult {
    let palette = "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0";
    let soc = TestSoc::new().entity(
        TestSocEntity::prop_choosing(
            "fixture",
            [0.0; 3],
            IDENTITY,
            "objects/test/lamp.cgf",
            Some("objects/test/lamp"),
            Some((palette, "left")),
        )
        .with_lights(&[("Light-1", TestLight::new("Omni"), IDENTITY, [0.0; 3])]),
    );

    let entities = read(&soc)?;

    let [prop] = entities.props.as_slice() else {
        return Err("one prop".into());
    };
    assert_eq!(prop.material.as_deref(), Some("objects/test/lamp"));
    let named = prop.palette.as_ref().ok_or("no palette")?;
    assert_eq!(
        (named.root.as_str(), named.child.as_str()),
        (palette, "left")
    );
    assert_eq!(entities.lights.len(), 1);
    assert_eq!(entities.classed.len(), 0);
    Ok(())
}

#[test]
fn an_empty_material_and_a_zero_palette_are_none() -> TestResult {
    let soc = TestSoc::new().entity(TestSocEntity::prop_choosing(
        "plain",
        [0.0; 3],
        IDENTITY,
        "objects/test/a.cgf",
        Some(""),
        Some(("00000000-0000-0000-0000-000000000000", "")),
    ));

    let entities = read(&soc)?;

    assert_eq!(entities.props[0].material, None);
    assert_eq!(entities.props[0].palette, None);
    Ok(())
}

#[test]
fn a_file_without_an_entity_chunk_yields_none_and_garbage_is_an_error() -> TestResult {
    assert!(SocEntities::read(&TestSoc::new().bytes())?.is_none());
    assert!(SocEntities::read(b"not a chunk file").is_err());
    assert!(SocEntities::parse(b"<Entities><Entity").is_err());
    Ok(())
}
