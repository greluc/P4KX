// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::Manifest;

/// Returns the JSON Schema (draft 2020-12) of `scene.json`, generated from [`Manifest`].
#[must_use]
pub fn manifest_schema() -> serde_json::Value {
    schemars::schema_for!(Manifest).to_value()
}
