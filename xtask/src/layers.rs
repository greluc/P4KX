use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Foundation,
    Format,
    Service,
    Frontend,
    Testkit,
    Tooling,
}

impl Layer {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "foundation" => Some(Self::Foundation),
            "format" => Some(Self::Format),
            "service" => Some(Self::Service),
            "frontend" => Some(Self::Frontend),
            "testkit" => Some(Self::Testkit),
            "tooling" => Some(Self::Tooling),
            _ => None,
        }
    }

    const fn may_depend_on(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Format | Self::Testkit, Self::Foundation)
                | (
                    Self::Service,
                    Self::Foundation | Self::Format | Self::Service
                )
                | (Self::Frontend, Self::Service)
        )
    }

    fn may_dev_depend_on(self, target: Self) -> bool {
        self.may_depend_on(target) || (target == Self::Testkit && self != Self::Testkit)
    }
}

struct Package {
    name: String,
    layer: Layer,
    dependencies: Vec<(String, bool)>,
}

/// Returns one message per workspace dependency that points against the layer direction.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let packages = workspace_packages(root)?;
    let layers: HashMap<&str, Layer> = packages
        .iter()
        .map(|package| (package.name.as_str(), package.layer))
        .collect();
    let mut violations = Vec::new();
    for package in &packages {
        for (dependency, dev) in &package.dependencies {
            let Some(&target) = layers.get(dependency.as_str()) else {
                continue;
            };
            let allowed = if *dev {
                package.layer.may_dev_depend_on(target)
            } else {
                package.layer.may_depend_on(target)
            };
            if !allowed {
                let kind = if *dev { "dev-dependency" } else { "dependency" };
                violations.push(format!(
                    "layer violation: {} ({:?}) has a {kind} on {dependency} ({target:?})",
                    package.name, package.layer
                ));
            }
        }
    }
    Ok(violations)
}

fn workspace_packages(root: &Path) -> Result<Vec<Package>> {
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
        .context("cargo metadata has no packages")?
        .iter()
        .map(package)
        .collect()
}

fn package(value: &Value) -> Result<Package> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .context("package without name")?
        .to_owned();
    let layer_name = value
        .pointer("/metadata/p4kx/layer")
        .and_then(Value::as_str)
        .with_context(|| format!("{name} has no [package.metadata.p4kx] layer"))?;
    let layer = Layer::parse(layer_name)
        .with_context(|| format!("{name} has an unknown layer `{layer_name}`"))?;
    let dependencies = value
        .get("dependencies")
        .and_then(Value::as_array)
        .context("package without dependencies")?
        .iter()
        .filter_map(|dependency| {
            let name = dependency.get("name").and_then(Value::as_str)?.to_owned();
            let dev = dependency.get("kind").and_then(Value::as_str) == Some("dev");
            Some((name, dev))
        })
        .collect();
    Ok(Package {
        name,
        layer,
        dependencies,
    })
}
