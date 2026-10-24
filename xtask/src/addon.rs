// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const MANIFEST: &str = "blender_addon/p4kx_importer/blender_manifest.toml";

/// Returns a message when the version of the Blender add-on manifest differs from the workspace
/// version, which is the Release version.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(root.join(MANIFEST))
        .with_context(|| format!("cannot read {MANIFEST}"))?;
    let workspace = crate::changelog::workspace_version(root)?;
    Ok(compare(&text, &workspace)?.into_iter().collect())
}

fn compare(manifest: &str, workspace: &str) -> Result<Option<String>> {
    let manifest: toml::Table =
        toml::from_str(manifest).with_context(|| format!("{MANIFEST} is not TOML"))?;
    let version = manifest
        .get("version")
        .and_then(toml::Value::as_str)
        .with_context(|| format!("{MANIFEST} has no version"))?;
    Ok((version != workspace).then(|| {
        format!(
            "{MANIFEST} has version {version}, the workspace version is {workspace}; set the manifest to {workspace}"
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::compare;

    #[test]
    fn equal_versions_pass() {
        assert_eq!(compare("version = \"0.4.0\"", "0.4.0").unwrap(), None);
    }

    #[test]
    fn a_differing_version_is_reported() {
        let message = compare("version = \"0.3.0\"", "0.4.0").unwrap().unwrap();
        assert!(message.contains("0.3.0") && message.contains("0.4.0"));
    }

    #[test]
    fn a_manifest_without_version_is_an_error() {
        assert!(compare("id = \"x\"", "0.4.0").is_err());
    }
}
