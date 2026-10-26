// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Parts whose port flags name a sub-geometry of a real Archive: the side doors of the Cutlass
//! Black and the right thrusters of the Prospector.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, LoadoutResolver, Part,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

use super::real_entity_interior::hull_bounds;

type TestResult = Result<(), Box<dyn Error>>;

fn assemble(archive: &Archive, class_name: &str) -> Result<Assembly, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(class_name)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(archive);
    Ok(Assembly::build(
        &tree,
        &models,
        &AssemblyOptions::default(),
    )?)
}

fn world_x(part: &Part) -> Option<(f32, f32)> {
    let bounds = hull_bounds(part.model()?)?;
    let rows = part.world().rows;
    let xs = [bounds.0, bounds.1].into_iter().flat_map(|corner_x| {
        [bounds.0, bounds.1].into_iter().flat_map(move |corner_y| {
            [bounds.0, bounds.1].into_iter().map(move |corner_z| {
                let [x, y, z] = [corner_x[0], corner_y[1], corner_z[2]];
                rows[0][0].mul_add(x, rows[0][1].mul_add(y, rows[0][2].mul_add(z, rows[0][3])))
            })
        })
    });
    xs.fold(None, |range, x| match range {
        None => Some((x, x)),
        Some((low, high)) => Some((low.min(x), high.max(x))),
    })
}

fn named<'a>(assembly: &'a Assembly, port: &str) -> Result<&'a Part, Box<dyn Error>> {
    assembly
        .parts()
        .iter()
        .find(|part| part.path().ends_with(port))
        .ok_or_else(|| format!("no part at a port {port}").into())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_cutlass_black_draws_both_side_doors_on_their_own_sides() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let assembly = assemble(&archive, "DRAK_Cutlass_Black")?;
    let left = named(&assembly, "hardpoint_door_left")?;
    let right = named(&assembly, "hardpoint_door_right")?;
    let (left_x, right_x) = (
        world_x(left).ok_or("the left door has no model")?,
        world_x(right).ok_or("the right door has no model")?,
    );
    assert!(
        left.geometry().is_some_and(|g| g.contains("left")),
        "{:?}",
        left.geometry()
    );
    assert!(
        right.geometry().is_some_and(|g| g.contains("right")),
        "{:?}",
        right.geometry()
    );
    assert!(left_x.1 < right_x.0, "left {left_x:?}, right {right_x:?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_prospector_right_thrusters_sit_on_the_right() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let assembly = assemble(&archive, "MISC_Prospector")?;
    for port in ["hardpoint_thruster_main_rear", "hardpoint_thruster_retro"] {
        let left = named(&assembly, &format!("{port}_left"))?;
        let right = named(&assembly, &format!("{port}_right"))?;
        let (left_x, right_x) = (
            world_x(left).ok_or("the left thruster has no model")?,
            world_x(right).ok_or("the right thruster has no model")?,
        );
        assert!(left_x.1 < 0.0, "{port} left {left_x:?}");
        assert!(right_x.0 > 0.0, "{port} right {right_x:?}");
    }
    Ok(())
}
