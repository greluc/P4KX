// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior of real ship containers: the models land inside the container's bounds.

use std::error::Error;

use p4kx_assembly::{Interior, InteriorOptions, InteriorPartKind, IssueKind};
use p4kx_extraction::{Archive, select_entries};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MANTIS_BODY: &str = "Data\\ObjectContainers\\Ships\\RSI\\Mantis\\base_int_body_main.socpak";
const MIN_BOUNDS: [f32; 3] = [-4.17, -0.64, -1.51];
const MAX_BOUNDS: [f32; 3] = [4.17, 15.85, 3.9];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_mantis_body_places_its_models_inside_the_bounds_the_container_declares() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let found = select_entries(&archive, &[MANTIS_BODY])?;
    let entry = found.first().ok_or("no Mantis body container")?;
    let root = archive.open_container(entry)?;

    let interior = Interior::build(
        &[&archive, &root],
        entry.path(),
        &InteriorOptions::default(),
    )?;

    assert!(interior.model_parts() >= 10, "{}", interior.model_parts());
    assert_eq!(interior.modelled_parts(), interior.model_parts());
    let mut inside = 0;
    for part in interior
        .parts()
        .iter()
        .filter(|part| part.kind().has_model())
    {
        let [x, y, z] = part.world().translation();
        let ok = [x, y, z]
            .iter()
            .zip(MIN_BOUNDS.iter().zip(&MAX_BOUNDS))
            .all(|(v, (min, max))| v >= min && v <= max);
        inside += usize::from(ok);
    }
    assert_eq!(
        inside,
        interior.model_parts(),
        "every model lies in the bounds"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_mantis_body_has_props_and_lights_inside_the_bounds_and_writes_a_clean_glb() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let found = select_entries(&archive, &[MANTIS_BODY])?;
    let entry = found.first().ok_or("no Mantis body container")?;
    let root = archive.open_container(entry)?;

    let interior = Interior::build(
        &[&archive, &root],
        entry.path(),
        &InteriorOptions::default(),
    )?;

    assert!(interior.props() >= 10, "{}", interior.props());
    assert!(interior.lights() >= 30, "{}", interior.lights());
    for part in interior
        .parts()
        .iter()
        .filter(|part| part.kind() == InteriorPartKind::Light)
    {
        let position = part.world().translation();
        let inside = position
            .iter()
            .zip(MIN_BOUNDS.iter().zip(&MAX_BOUNDS))
            .all(|(v, (min, max))| *v >= min - 1.0 && *v <= max + 1.0);
        assert!(inside, "{} at {position:?}", part.name());
    }
    let mut bytes = Vec::new();
    let summary = interior.write_glb(&GlbOptions::default(), &mut bytes)?;
    let glb = check_glb(&bytes)?;
    assert_eq!(summary.lights, interior.lights());
    assert_eq!(
        glb.document.lights().map_or(0, Iterator::count),
        interior.lights()
    );
    assert!(
        interior
            .issues()
            .iter()
            .all(|issue| matches!(issue.kind(), IssueKind::LightUnsupported { .. })),
        "{:?}",
        interior.issues()
    );
    Ok(())
}
