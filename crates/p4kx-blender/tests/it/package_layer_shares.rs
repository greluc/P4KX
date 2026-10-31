// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The layer shares a package states for the layered material of `TEST_Layered`.

use crate::package_materials::write;
use crate::support::TestResult;

#[test]
fn a_layered_material_states_the_share_of_each_layer() -> TestResult {
    let written = write()?;
    let hull = written.material("hull")?;

    // `WearBlendBase` is 0.25 and the third layer's file is missing: the first layer weighs
    // 0.75, the two others 0.25 / 3 each.
    let [first, second, lost, third] = hull.layer_shares.as_slice() else {
        return Err(format!("{} shares", hull.layer_shares.len()).into());
    };
    let total = 0.75_f32 + 2.0 * 0.25 / 3.0;
    assert!((first - 0.75 / total).abs() < 1e-6, "first {first}");
    assert!(
        (second - 0.25 / 3.0 / total).abs() < 1e-6,
        "second {second}"
    );
    assert!((third - second).abs() < 1e-6);
    assert!(lost.abs() < f32::EPSILON, "a missing layer has no share");
    assert_eq!(written.material("canopy")?.layer_shares, Vec::<f32>::new());
    Ok(())
}
