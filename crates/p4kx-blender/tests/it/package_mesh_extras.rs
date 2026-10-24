// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The glTF material index each package mesh carries in the `extras` of its materials.

use crate::package_materials::write;
use crate::support::TestResult;

#[test]
fn every_glb_material_carries_its_index_in_its_extras() -> TestResult {
    let written = write()?;

    for mesh in &written.manifest.meshes {
        let bytes = std::fs::read(written.dir.path().join("ship").join(&mesh.file))?;
        p4kx_testkit::check_glb(&bytes)?;
        let length = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
        let document: serde_json::Value = serde_json::from_slice(&bytes[20..20 + length])?;
        let materials = document["materials"].as_array().ok_or("materials")?;
        assert_eq!(materials.len(), mesh.materials.len());
        for (index, material) in materials.iter().enumerate() {
            assert_eq!(material["extras"]["p4kx_index"], index);
            assert_eq!(mesh.materials[index].index, index);
        }
    }
    Ok(())
}
