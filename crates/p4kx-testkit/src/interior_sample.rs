// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    SOC_IDENTITY, TestContainer, TestEntry, TestModel, TestMtl, TestMtlMaterial,
    TestObjectContainer, TestSoc,
};

/// The Entry of the sample Interior's root container.
pub const INTERIOR_ROOT: &str = "Data\\ObjectContainers\\ships\\test\\ship_interior.socpak";

/// The path of the root container's Entry as a nested container may name it.
pub const INTERIOR_ROOT_NAME: &str = "ObjectContainers/ships/test/ship_interior";

const HALF_TURN: f64 = std::f64::consts::FRAC_1_SQRT_2;

const fn at(x: f64, y: f64, z: f64) -> [[f64; 4]; 3] {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

fn model_entries(path: &str, model: &TestModel) -> Vec<TestEntry> {
    let mut entries = vec![
        TestEntry::stored(format!("{path}.cgf"), model.cgf()),
        TestEntry::stored(format!("{path}.cgfm"), model.cgfm()),
    ];
    if let (Some(cgf), Some(cgfm)) = (model.lod_cgf(1), model.lod_cgfm(1)) {
        entries.push(TestEntry::stored(format!("{path}_lod1.cgf"), cgf));
        entries.push(TestEntry::stored(format!("{path}_lod1.cgfm"), cgfm));
    }
    entries
}

fn cockpit() -> TestContainer {
    let soc = TestSoc::new()
        .model("objects/test/cockpit_seat.cgf")
        .model("objects/test/cube.cgf")
        .object(0, at(0.5, 0.0, 0.0))
        .object(1, at(1.0, 0.0, 0.0));
    let description = TestObjectContainer::new("cockpit")
        .child(INTERIOR_ROOT_NAME, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0])
        .stream("cockpit.soc");
    let mut container = TestContainer::new()
        .entry(TestEntry::stored("cockpit.xml", description.cryxml()))
        .entry(TestEntry::deflated("cockpit.soc", soc.bytes()));
    for entry in model_entries(
        "Objects\\Test\\cockpit_seat",
        &TestModel::cube().scaled(0.5),
    ) {
        container = container.entry(entry);
    }
    container
}

fn ring(name: &str, next: &str) -> TestContainer {
    let description =
        TestObjectContainer::new(name).child(next, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]);
    TestContainer::new().entry(TestEntry::stored(format!("{name}.xml"), description.xml()))
}

fn root() -> TestContainer {
    let soc = TestSoc::new()
        .model("objects/test/cube.cgf")
        .model("objects/test/lodded.cgf")
        .model("objects/test/absent.cgf")
        .object(0, at(1.0, 0.0, 0.0))
        .object(0, at(0.0, 0.0, 2.0))
        .object(1, at(0.0, 3.0, 0.0))
        .object(2, SOC_IDENTITY);
    let description = TestObjectContainer::new("ship_interior")
        .child(
            "modules/cockpit",
            [0.0, 5.0, 0.0],
            [HALF_TURN, 0.0, 0.0, HALF_TURN],
        )
        .child("modules/absent", [0.0; 3], [1.0, 0.0, 0.0, 0.0])
        .child("modules/ring_a", [0.0, -5.0, 0.0], [1.0, 0.0, 0.0, 0.0])
        .raw_child(&[("name", "modules/cockpit"), ("pos", "1,2")])
        .stream("ship_interior.soc");
    TestContainer::new()
        .entry(TestEntry::stored("ship_interior.xml", description.xml()))
        .entry(TestEntry::deflated("ship_interior.soc", soc.bytes()))
        .container("modules\\cockpit.socpak", &cockpit())
        .container("modules\\ring_a.socpak", &ring("ring_a", "modules/ring_b"))
        .container("modules\\ring_b.socpak", &ring("ring_b", "modules/ring_a"))
}

/// Returns the Entries of a synthetic Archive that holds a ship Interior as a socpak container,
/// [`INTERIOR_ROOT`].
///
/// The root container places `cube` twice, `lodded` and `absent` (which the Archive lacks), and
/// nests `modules/cockpit` at (0, 5, 0) turned a quarter about Z, `modules/absent` (which does not
/// exist), `modules/ring_a` at (0, -5, 0) and a container whose placement is malformed. The cockpit
/// places its own `cockpit_seat` model, which only the container holds, and `cube`, and nests the
/// root again; `ring_a` and `ring_b` nest each other. The cockpit's description is `CryXml`, the
/// others are text XML. `cube` names the library `objects/test/ship`, red; `lodded` has a second
/// level of detail at half the size.
#[must_use]
pub fn interior_entries() -> Vec<TestEntry> {
    let mut entries = Vec::new();
    entries.extend(model_entries(
        "Data\\Objects\\Test\\cube",
        &TestModel::cube().with_material_library("objects/test/ship"),
    ));
    entries.extend(model_entries(
        "Data\\Objects\\Test\\lodded",
        &TestModel::cube().with_lod(TestModel::cube().scaled(0.5)),
    ));
    let material = |name: &str| TestMtlMaterial::new(name).diffuse([0.8, 0.1, 0.1]);
    entries.push(TestEntry::stored(
        "Data\\objects\\test\\ship.mtl",
        TestMtl::multi(&[material("unused"), material("caps"), material("sides")]).cryxml(),
    ));
    entries.push(TestEntry::stored(INTERIOR_ROOT, root().build()));
    entries
}
