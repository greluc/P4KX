// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use toml::{Table, Value};

use crate::file_size::collect_rust_files;

const UNSAFE_PACKAGE: &str = "p4kx-gui";
const UNSAFE_MODULE: &str = "crates/p4kx-gui/src/dwm.rs";

/// Returns one message per package whose lints are not the workspace lints, per source file
/// outside the DWM module that names `unsafe_code`, and per profile that aborts on panic.
///
/// Only `p4kx-gui` has its own table: the workspace lints with `unsafe_code` lowered from
/// `forbid` to `deny`, so its DWM module can expect it.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let workspace = read(&root.join("Cargo.toml"))?;
    let lints = workspace
        .get("workspace")
        .and_then(|workspace| workspace.get("lints"))
        .and_then(Value::as_table)
        .context("Cargo.toml has no [workspace.lints]")?;
    let mut violations = Vec::new();
    for (name, manifest) in manifests(root)? {
        let package = read(&manifest)?;
        if let Some(violation) = violation(&name, package.get("lints"), lints) {
            violations.push(violation);
        }
    }
    violations.extend(unsafe_sites(root)?);
    violations.extend(aborting_profiles("Cargo.toml", &workspace));
    let config = root.join(".cargo").join("config.toml");
    if config.exists() {
        violations.extend(aborting_profiles(".cargo/config.toml", &read(&config)?));
    }
    Ok(violations)
}

/// Returns one message per profile of `table` with `panic = "abort"`; the Vorbis validation and
/// decoding and the JPEG decoding catch decoder panics, which needs unwinding (ADR-0011, ADR-0013,
/// ADR-0015).
fn aborting_profiles(file: &str, table: &Table) -> Vec<String> {
    table
        .get("profile")
        .and_then(Value::as_table)
        .into_iter()
        .flatten()
        .filter(|(_, profile)| profile.get("panic").and_then(Value::as_str) == Some("abort"))
        .map(|(name, _)| {
            format!("lints: {file} sets panic = \"abort\" in profile {name}; keep unwinding")
        })
        .collect()
}

fn violation(name: &str, lints: Option<&Value>, workspace: &Table) -> Option<String> {
    let (expected, rule) = if name == UNSAFE_PACKAGE {
        let mut own = workspace.clone();
        if let Some(rust) = own.get_mut("rust").and_then(Value::as_table_mut) {
            rust.insert("unsafe_code".to_owned(), Value::from("deny"));
        }
        (
            own,
            "must repeat [workspace.lints] with only unsafe_code = \"deny\" changed",
        )
    } else {
        let mut inherit = Table::new();
        inherit.insert("workspace".to_owned(), Value::Boolean(true));
        (
            inherit,
            "must use the workspace lints ([lints] workspace = true)",
        )
    };
    (lints != Some(&Value::Table(expected))).then(|| format!("lints: {name} {rule}"))
}

fn manifests(root: &Path) -> Result<Vec<(String, PathBuf)>> {
    crate::metadata::packages(root)?
        .iter()
        .map(|package| {
            let name = package
                .get("name")
                .and_then(serde_json::Value::as_str)
                .context("package without name")?;
            let manifest = package
                .get("manifest_path")
                .and_then(serde_json::Value::as_str)
                .with_context(|| format!("{name} has no manifest path"))?;
            Ok((name.to_owned(), PathBuf::from(manifest)))
        })
        .collect()
}

fn unsafe_sites(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    collect_rust_files(&root.join("crates"), &mut files)?;
    let mut violations = Vec::new();
    for file in files {
        let shown = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
        let text =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        if names_unsafe_code(&shown, &text) {
            violations.push(format!(
                "lints: {} names unsafe_code; only {UNSAFE_MODULE} may",
                shown.display()
            ));
        }
    }
    Ok(violations)
}

fn names_unsafe_code(relative: &Path, text: &str) -> bool {
    relative != Path::new(UNSAFE_MODULE) && text.contains("unsafe_code")
}

fn read(manifest: &Path) -> Result<Table> {
    let text = fs::read_to_string(manifest)
        .with_context(|| format!("cannot read {}", manifest.display()))?;
    text.parse()
        .with_context(|| format!("cannot parse {}", manifest.display()))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use toml::{Table, Value};

    use super::{aborting_profiles, names_unsafe_code, violation};

    type TestResult = Result<(), toml::de::Error>;

    fn workspace() -> Result<Table, toml::de::Error> {
        r#"
            rust = { unsafe_code = "forbid", missing_docs = "deny" }
            clippy = { unwrap_used = "deny" }
        "#
        .parse()
    }

    fn lints(text: &str) -> Result<Value, toml::de::Error> {
        Ok(Value::Table(text.parse()?))
    }

    #[test]
    fn a_package_inheriting_the_workspace_lints_passes() -> TestResult {
        let inherited = lints("workspace = true")?;

        assert_eq!(
            violation("p4kx-archive", Some(&inherited), &workspace()?),
            None,
            "inherits"
        );
        Ok(())
    }

    #[test]
    fn a_package_with_its_own_or_no_lints_fails() -> TestResult {
        let own = lints(r#"rust = { unsafe_code = "deny", missing_docs = "deny" }"#)?;

        assert!(
            violation("p4kx-archive", Some(&own), &workspace()?).is_some(),
            "own table"
        );
        assert!(
            violation("p4kx-archive", None, &workspace()?).is_some(),
            "no table"
        );
        Ok(())
    }

    #[test]
    fn the_gui_may_only_lower_unsafe_code_to_deny() -> TestResult {
        let lowered = lints(
            r#"
                rust = { unsafe_code = "deny", missing_docs = "deny" }
                clippy = { unwrap_used = "deny" }
            "#,
        )?;
        let loosened = lints(
            r#"
                rust = { unsafe_code = "deny", missing_docs = "deny" }
                clippy = { unwrap_used = "warn" }
            "#,
        )?;

        assert_eq!(
            violation("p4kx-gui", Some(&lowered), &workspace()?),
            None,
            "only unsafe_code lowered"
        );
        assert!(
            violation("p4kx-gui", Some(&loosened), &workspace()?).is_some(),
            "another lint loosened"
        );
        assert!(
            violation("p4kx-gui", Some(&lints("workspace = true")?), &workspace()?).is_some(),
            "forbid would reject the DWM module"
        );
        Ok(())
    }

    #[test]
    fn a_profile_that_aborts_on_panic_fails() -> TestResult {
        let unwinding: Table = r#"
            [profile.release]
            lto = "thin"
            panic = "unwind"
        "#
        .parse()?;
        let aborting: Table = r#"
            [profile.release]
            lto = "thin"
            [profile.dist]
            inherits = "release"
            panic = "abort"
        "#
        .parse()?;

        assert!(
            aborting_profiles("Cargo.toml", &unwinding).is_empty(),
            "unwinding profiles"
        );
        assert_eq!(
            aborting_profiles("Cargo.toml", &aborting),
            ["lints: Cargo.toml sets panic = \"abort\" in profile dist; keep unwinding"],
            "the aborting profile"
        );
        Ok(())
    }

    #[test]
    fn only_the_dwm_module_may_name_unsafe_code() {
        let expectation = "#![expect(unsafe_code, reason = \"FFI\")]";

        assert!(
            !names_unsafe_code(Path::new("crates/p4kx-gui/src/dwm.rs"), expectation),
            "the DWM module"
        );
        assert!(
            names_unsafe_code(Path::new("crates/p4kx-gui/src/app.rs"), expectation),
            "another module"
        );
        assert!(
            !names_unsafe_code(Path::new("crates/p4kx-gui/src/app.rs"), "fn safe() {}"),
            "no mention"
        );
    }
}
