// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::interior_sample::model_entries;
use crate::{
    DataForgeBuilder, TestContainer, TestEntity, TestEntry, TestLight, TestModel, TestMtl,
    TestMtlMaterial, TestObjectContainer, TestSoc, TestSocEntity, entity_schema,
    tint_palette_record,
};

/// The Entry of the container of [`class_room_entries`].
pub const CLASS_ROOM: &str = "Data\\ObjectContainers\\ships\\test\\class_room.socpak";

/// The class name of the ship Entity of [`entity_class_room_entries`].
pub const ENTITY_CLASS_ROOM_CLASS: &str = "TEST_ClassHull";

/// The sRGB bytes of the tint colours A to C of the palette `p_door`.
pub const CLASS_DOOR_TINTS: [[u8; 3]; 3] = [[255, 0, 0], [0, 255, 0], [0, 0, 255]];

/// The sRGB bytes of the tint colours A to C of the palette `p_own`.
pub const CLASS_OWN_TINTS: [[u8; 3]; 3] = [[0, 255, 255], [255, 0, 255], [255, 255, 0]];

/// The sRGB bytes of the tint colours A to C of the palette `p_room`.
pub const CLASS_ROOM_TINTS: [[u8; 3]; 3] = [[255, 128, 0], [128, 255, 0], [0, 128, 255]];

/// The sRGB bytes of the tint colours A to C of `p_default`, the default palette of the library
/// the cabinets are drawn with.
pub const CLASS_DEFAULT_TINTS: [[u8; 3]; 3] = [[10, 20, 30], [40, 50, 60], [70, 80, 90]];

/// The GUID text of the class Record `TEST_Door`.
pub const CLASS_DOOR_GUID: &str = "00000000-0000-0000-0000-000000000041";

/// The GUID text of the class Record `TEST_Cabinet`.
pub const CLASS_CABINET_GUID: &str = "00000000-0000-0000-0000-000000000042";

const MARKER_GUID: &str = "00000000-0000-0000-0000-000000000043";
const FIXTURE_GUID: &str = "00000000-0000-0000-0000-000000000044";
const UNKNOWN_GUID: &str = "00000000-0000-0000-0000-00000000004f";
const OWN_PALETTE_GUID: &str = "00000000-0000-0000-0000-000000000052";
const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];
const ROOM_PALETTE: &str = "Libs/Foundry/Records/TintPalettes/brand/test/p_room";
const MISSING_PALETTE: &str = "Libs/Foundry/Records/TintPalettes/brand/test/p_missing";
const DEFAULT_PALETTE: &str = "Libs/Foundry/Records/TintPalettes/brand/test/p_default";

const fn at(x: f64, y: f64, z: f64) -> [[f64; 4]; 3] {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

fn database(hull: bool) -> DataForgeBuilder {
    let class = |guid: u128, name: &str| {
        TestEntity::new(
            guid,
            name,
            format!("entities/scitem/{}.xml", name.to_ascii_lowercase()),
        )
    };
    let mut entities = vec![
        class(0x41, "TEST_Door")
            .geometry("Objects/Test/cube.cgf", Some("Objects/Test/painted"))
            .palette(0x51, ""),
        class(0x42, "TEST_Cabinet").geometry(
            "Objects/Test/cube.cgf",
            Some("Objects/Test/painted_default"),
        ),
        class(0x43, "TEST_Marker"),
        class(0x44, "TEST_Fixture").geometry("Objects/Test/cube.cgf", None),
    ];
    if hull {
        entities.push(
            class(0x1, ENTITY_CLASS_ROOM_CLASS)
                .geometry("Objects/Test/cube.cgf", None)
                .object_container(
                    CLASS_ROOM.trim_start_matches("Data\\"),
                    "",
                    [0.0; 3],
                    [0.0; 3],
                ),
        );
    }
    let palettes = [
        tint_palette_record(0x51, "p_door", CLASS_DOOR_TINTS, [255; 3], [51, 102, 153]),
        tint_palette_record(0x52, "p_own", CLASS_OWN_TINTS, [255; 3], [51, 102, 153]),
        tint_palette_record(0x53, "p_room", CLASS_ROOM_TINTS, [255; 3], [51, 102, 153]),
        tint_palette_record(
            0x54,
            "p_default",
            CLASS_DEFAULT_TINTS,
            [255; 3],
            [51, 102, 153],
        ),
    ];
    let builder = entities
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        });
    palettes.into_iter().fold(builder, DataForgeBuilder::record)
}

fn library(default_palette: Option<&str>) -> Vec<u8> {
    let painted = |name: &str, tint: &str| {
        TestMtlMaterial::new(name)
            .shader("HardSurface")
            .diffuse([0.5; 3])
            .layer("Primary", tint)
    };
    let file = TestMtl::multi(&[
        TestMtlMaterial::new("unused"),
        painted("caps", "1"),
        painted("sides", "2"),
    ]);
    match default_palette {
        Some(palette) => file.with_default_palette(palette).cryxml(),
        None => file.cryxml(),
    }
}

fn classed(name: &str, class: &str, guid: &str, position: [f64; 3]) -> TestSocEntity {
    TestSocEntity::classed(name, class, guid, position, IDENTITY)
}

fn soc() -> TestSoc {
    let lamp = [("Light-A", TestLight::new("Omni"), IDENTITY, [0.0, 0.0, 1.0])];
    TestSoc::new()
        .model("objects/test/cube.cgf")
        .palette(ROOM_PALETTE)
        .palette(MISSING_PALETTE)
        .object_with(0, at(4.0, 0.0, 0.0), 0)
        .object_with(0, at(5.0, 0.0, 0.0), 1)
        .entity(classed("door", "Door", CLASS_DOOR_GUID, [1.0, 0.0, 0.0]))
        .entity(classed(
            "cabinet_a",
            "Cabinet",
            CLASS_CABINET_GUID,
            [2.0, 0.0, 0.0],
        ))
        .entity(classed(
            "cabinet_b",
            "Cabinet",
            CLASS_CABINET_GUID,
            [3.0, 0.0, 0.0],
        ))
        .entity(
            classed("door_own", "Door", CLASS_DOOR_GUID, [8.0, 0.0, 0.0]).choosing(
                Some("Objects/Test/painted_default"),
                Some((OWN_PALETTE_GUID, "")),
            ),
        )
        .entity(classed("marker", "Marker", MARKER_GUID, [0.0; 3]))
        .entity(classed("stranger", "Stranger", UNKNOWN_GUID, [0.0; 3]))
        .entity(classed("bad_door", "Door", CLASS_DOOR_GUID, [0.0; 3]).raw_pos("1,x,3"))
        .entity(
            TestSocEntity::classed(
                "lamp_fixture",
                "LightGroupPoweredItem",
                FIXTURE_GUID,
                [0.0, 6.0, 0.0],
                IDENTITY,
            )
            .with_lights(&lamp),
        )
        .entity(TestSocEntity::prop_choosing(
            "own",
            [0.0, 7.0, 0.0],
            IDENTITY,
            "objects/test/cube.cgf",
            Some("Objects/Test/painted"),
            Some((OWN_PALETTE_GUID, "")),
        ))
}

/// Returns the Entries of a synthetic Archive with a DataForge and a room as a socpak container,
/// [`CLASS_ROOM`], whose `.soc` file places
///
/// - `door` (at (1, 0, 0)) of the class `TEST_Door`, whose Record names the cube, the library
///   `Objects/Test/painted` and the palette `p_door` ([`CLASS_DOOR_TINTS`]);
/// - `door_own` (at (8, 0, 0)) of the same class, which chooses the library
///   `Objects/Test/painted_default` and the palette `p_own` ([`CLASS_OWN_TINTS`]) itself;
/// - `cabinet_a` and `cabinet_b` (at (2, 0, 0) and (3, 0, 0)) of the class `TEST_Cabinet`, whose
///   library `Objects/Test/painted_default` names `p_default` ([`CLASS_DEFAULT_TINTS`]) as its
///   default;
/// - `marker` of a class without geometry, `stranger` of a class the DataForge lacks and
///   `bad_door` of `TEST_Door` with an unusable position;
/// - `lamp_fixture` (at (0, 6, 0)) of the class `TEST_Fixture` with a geometry and a baked-in
///   `Omni` light `Light-A`;
/// - `own` (at (0, 7, 0)), a prop that names the cube, the library `Objects/Test/painted` and the
///   palette `p_own` ([`CLASS_OWN_TINTS`]) itself;
/// - two cubes from the object table, at (4, 0, 0) with the palette path of `p_room`
///   ([`CLASS_ROOM_TINTS`]) and at (5, 0, 0) with a palette path no Record has.
#[must_use]
pub fn class_room_entries() -> Vec<TestEntry> {
    entries(false)
}

/// Returns the Entries of [`class_room_entries`] and the ship Entity [`ENTITY_CLASS_ROOM_CLASS`],
/// a cube whose Record names the room as its object container at the origin.
#[must_use]
pub fn entity_class_room_entries() -> Vec<TestEntry> {
    let mut entries = entries(true);
    let hull = TestModel::cube();
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\hull.cgf",
        hull.cgf(),
    ));
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\hull.cgfm",
        hull.cgfm(),
    ));
    entries
}

fn entries(hull: bool) -> Vec<TestEntry> {
    let description = TestObjectContainer::new("class_room").stream("class_room.soc");
    let container = TestContainer::new()
        .entry(TestEntry::stored("class_room.xml", description.cryxml()))
        .entry(TestEntry::deflated("class_room.soc", soc().bytes()));
    let mut entries = model_entries("Data\\Objects\\Test\\cube", &TestModel::cube());
    entries.push(TestEntry::zstd("Data\\Game2.dcb", database(hull).build()));
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\painted.mtl",
        library(None),
    ));
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\painted_default.mtl",
        library(Some(DEFAULT_PALETTE)),
    ));
    entries.push(TestEntry::stored(CLASS_ROOM, container.build()));
    entries
}
