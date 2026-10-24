// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The camera of the vehicle hologram, read from the `UIHoloVehicle_Config` Record.

use crate::assembly_support::{TestResult, entries, fixture};
use p4kx_assembly::HologramCamera;
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::hologram_entries;

fn read(
    entries: Vec<p4kx_testkit::TestEntry>,
) -> Result<Option<HologramCamera>, Box<dyn std::error::Error>> {
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    Ok(HologramCamera::read(&forge))
}

#[test]
fn the_record_sets_the_field_of_view_the_view_angle_and_the_distance_scaler() -> TestResult {
    let camera = read(hologram_entries([14.0, 45.0, 2.5]))?;

    assert_eq!(
        camera,
        Some(HologramCamera::new(14.0, 45.0, 2.5)),
        "the three fields of the Record"
    );
    Ok(())
}

#[test]
fn a_database_without_the_record_has_no_camera() -> TestResult {
    assert_eq!(read(entries())?, None, "no UIHoloVehicle_Config");
    Ok(())
}

#[test]
fn the_default_camera_is_what_the_current_game_sets() {
    let camera = HologramCamera::default();

    assert_eq!(camera.fov_degrees().to_bits(), 10.0_f32.to_bits(), "fov");
    assert_eq!(
        camera.view_angle_degrees().to_bits(),
        60.0_f32.to_bits(),
        "angle"
    );
    assert_eq!(
        camera.distance_scaler().to_bits(),
        1.5_f32.to_bits(),
        "scaler"
    );
}
