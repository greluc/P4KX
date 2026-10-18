use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Returns the `packages` of `cargo metadata` for the workspace at `root`, without dependencies.
pub(crate) fn packages(root: &Path) -> Result<Vec<Value>> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root)
        .output()
        .context("cannot run cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)?;
    metadata
        .get("packages")
        .and_then(Value::as_array)
        .cloned()
        .context("cargo metadata has no packages")
}
