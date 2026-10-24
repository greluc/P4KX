// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Documented ships with Interiors assembled from a real Archive: the GLB is valid, the rooms
//! sit inside the hull and little stays unresolved.

use std::error::Error;
use std::time::Instant;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, EntityInterior, InteriorOptions,
    IssueKind, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

use super::real_entity_interior::placement_against_hull;

type TestResult = Result<(), Box<dyn Error>>;

struct Ship {
    class_name: &'static str,
    size: &'static str,
    least_containers: usize,
    least_models: usize,
}

const SHIPS: [Ship; 5] = [
    Ship {
        class_name: "DRAK_Cutlass_Black",
        size: "small",
        least_containers: 1,
        least_models: 5,
    },
    Ship {
        class_name: "RSI_Constellation_Andromeda",
        size: "medium",
        least_containers: 2,
        least_models: 50,
    },
    Ship {
        class_name: "MISC_Freelancer",
        size: "medium",
        least_containers: 1,
        least_models: 20,
    },
    Ship {
        class_name: "AEGS_Idris_P",
        size: "capital",
        least_containers: 1,
        least_models: 100,
    },
    Ship {
        class_name: "RSI_Polaris",
        size: "capital",
        least_containers: 1,
        least_models: 100,
    },
];

const MAX_UNRESOLVED_SHARE_PERCENT: usize = 10;
const MAX_OUTSIDE_SHARE_PERCENT: usize = 1;

struct Built {
    assembly: Assembly,
    glb: Vec<u8>,
    seconds: f32,
}

fn build(archive: &Archive, class_name: &str) -> Result<Built, Box<dyn Error>> {
    let started = Instant::now();
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(archive);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let record = resolver.entities().find(class_name)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
    let interior = EntityInterior::build(archive, &assembly, &tree, &InteriorOptions::default())?;
    let assembly = assembly.with_interior(interior);
    let mut glb = Vec::new();
    assembly.write_glb(&GlbOptions::default(), &mut glb)?;
    Ok(Built {
        assembly,
        glb,
        seconds: started.elapsed().as_secs_f32(),
    })
}

fn open() -> Result<Archive, Box<dyn Error>> {
    Ok(Archive::open_with_key(
        real_archive_path()?,
        Some(real_archive_key()?),
    )?)
}

fn interior_of(built: &Built) -> Result<&EntityInterior, Box<dyn Error>> {
    Ok(built.assembly.interior().ok_or("no Interior")?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts go into the PR body")]
fn ships_with_interiors_write_valid_glbs_with_rooms_inside_the_hull() -> TestResult {
    let archive = open()?;
    for ship in &SHIPS {
        let built = build(&archive, ship.class_name)?;
        let checked = check_glb(&built.glb)?;
        let interior = interior_of(&built)?;
        let (placed, outside) = placement_against_hull(ship.class_name, &built.assembly, interior)?;
        let unresolved = interior.model_parts() - interior.modelled_parts();
        println!(
            "{} ({}): {:.1} s, {} bytes, {} nodes, {} containers, {} parts, {} models of which {} unresolved, {} lights, {} placed, {} outside",
            ship.class_name,
            ship.size,
            built.seconds,
            built.glb.len(),
            checked.document.nodes().count(),
            interior.containers().len(),
            interior.parts(),
            interior.model_parts(),
            unresolved,
            interior.lights(),
            placed,
            outside,
        );
        assert!(
            interior.containers().len() >= ship.least_containers,
            "{}: {} containers",
            ship.class_name,
            interior.containers().len()
        );
        assert!(
            placed >= ship.least_models,
            "{}: {placed} models placed",
            ship.class_name
        );
        assert!(
            outside * 100 <= placed * MAX_OUTSIDE_SHARE_PERCENT,
            "{}: {outside} of {placed} models outside the hull",
            ship.class_name
        );
        assert!(
            unresolved * 100 <= interior.model_parts() * MAX_UNRESOLVED_SHARE_PERCENT,
            "{}: {unresolved} of {} models unresolved",
            ship.class_name,
            interior.model_parts()
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_interiors_of_the_ships_have_no_cycles_and_no_missing_containers() -> TestResult {
    let archive = open()?;
    for ship in &SHIPS {
        let built = build(&archive, ship.class_name)?;
        let bad: Vec<_> = interior_of(&built)?
            .issues()
            .iter()
            .filter(|issue| {
                matches!(
                    issue.kind(),
                    IssueKind::ContainerCycle { .. }
                        | IssueKind::ContainerMissing { .. }
                        | IssueKind::ContainerUnreadable { .. }
                        | IssueKind::InvalidOffset { .. }
                        | IssueKind::TooManyContainers { .. }
                        | IssueKind::TooManyParts { .. }
                        | IssueKind::TooManyVertices { .. }
                )
            })
            .take(5)
            .collect();
        assert!(bad.is_empty(), "{}: {bad:?}", ship.class_name);
    }
    Ok(())
}
