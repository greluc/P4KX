// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json::Root;
use gltf_json::extras::RawValue;

/// Gives every material of `root` the `extras` `{"p4kx_index": <index in the file>}`.
pub(crate) fn index_materials(root: &mut Root) {
    for (index, material) in root.materials.iter_mut().enumerate() {
        material.extras = RawValue::from_string(format!("{{\"p4kx_index\":{index}}}")).ok();
    }
}
