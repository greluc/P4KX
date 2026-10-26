// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior of real ship containers: the parts land inside the bounds the container declares.

use std::error::Error;
use std::io::Read;

use p4kx_assembly::{Interior, InteriorOptions, InteriorPartKind, IssueKind};
use p4kx_extraction::{Archive, select_entries};
use p4kx_gltf::GlbOptions;
use p4kx_model::{ContainerBounds, ObjectContainer};
use p4kx_testkit::{check_glb, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MANTIS_BODY: &str = "Data\\ObjectContainers\\Ships\\RSI\\Mantis\\base_int_body_main.socpak";
const IDRIS_FRONT: &str =
    "Data\\ObjectContainers\\Ships\\AEGS\\Idris\\base_int_mdfront_main.socpak";
const TOLERANCE: f64 = 0.01;

fn built(path: &str) -> Result<(Interior, ContainerBounds), Box<dyn Error>> {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let found = select_entries(&archive, &[path])?;
    let entry = found.first().ok_or("no container")?;
    let root = archive.open_container(entry)?;
    let stem = path
        .rsplit('\\')
        .next()
        .and_then(|name| name.strip_suffix(".socpak"))
        .ok_or("no container name")?;
    let description = root
        .entries()
        .iter()
        .find(|file| file.path().eq_ignore_ascii_case(&format!("{stem}.xml")))
        .ok_or("no description")?;
    let mut bytes = Vec::new();
    root.open_entry(description)?.read_to_end(&mut bytes)?;
    let bounds = ObjectContainer::parse(&bytes)?
        .bounds
        .ok_or("the container declares no bounds")?;
    let interior = Interior::build(
        &[&archive, &root],
        entry.path(),
        &InteriorOptions::default(),
    )?;
    Ok((interior, bounds))
}

fn outside(interior: &Interior, bounds: &ContainerBounds, kind: InteriorPartKind) -> Vec<String> {
    interior
        .parts()
        .iter()
        .filter(|part| part.kind() == kind)
        .filter(|part| !bounds.contains(part.world().translation(), TOLERANCE))
        .take(5)
        .map(|part| format!("{} at {:?}", part.name(), part.world().translation()))
        .collect()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_mantis_body_places_its_models_inside_the_bounds_the_container_declares() -> TestResult {
    let (interior, bounds) = built(MANTIS_BODY)?;

    assert!(interior.model_parts() >= 10, "{}", interior.model_parts());
    assert_eq!(interior.modelled_parts(), interior.model_parts());
    let strays = outside(&interior, &bounds, InteriorPartKind::Model);
    assert!(strays.is_empty(), "{strays:?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_idris_front_places_its_models_and_props_inside_the_bounds_the_container_declares()
-> TestResult {
    let (interior, bounds) = built(IDRIS_FRONT)?;

    assert!(interior.model_parts() >= 1000, "{}", interior.model_parts());
    for kind in [InteriorPartKind::Model, InteriorPartKind::Prop] {
        let strays = outside(&interior, &bounds, kind);
        assert!(strays.is_empty(), "{kind:?}: {strays:?}");
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_mantis_body_has_props_and_lights_inside_the_bounds_and_writes_a_clean_glb() -> TestResult {
    let (interior, bounds) = built(MANTIS_BODY)?;

    assert!(interior.props() >= 10, "{}", interior.props());
    assert!(interior.lights() >= 30, "{}", interior.lights());
    let strays = outside(&interior, &bounds, InteriorPartKind::Light);
    assert!(strays.is_empty(), "{strays:?}");
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
