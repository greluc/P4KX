// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

const ARCHIVE: &str = "p4kx-archive";

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
            (Self::Format, Self::Foundation | Self::Format)
                | (Self::Testkit, Self::Foundation)
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

    fn allows(self, kind: DependencyKind, target: Self) -> bool {
        match kind {
            DependencyKind::Normal => self.may_depend_on(target),
            DependencyKind::Dev => self.may_dev_depend_on(target),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyKind {
    Normal,
    Dev,
}

struct Package {
    name: String,
    layer: Layer,
    builds_on: Vec<String>,
    dependencies: Vec<(String, DependencyKind)>,
}

impl Package {
    /// Whether this package may have a `kind` dependency on `target` in layer `target_layer`.
    ///
    /// A format crate depends on another format crate only when it names it in `builds-on`,
    /// and never on `p4kx-archive` (ADR-0019). A dev-dependency of a package on itself only turns
    /// on its test-only features and is always allowed.
    fn may_depend_on(&self, kind: DependencyKind, target: &str, target_layer: Layer) -> bool {
        if kind == DependencyKind::Dev && target == self.name {
            return true;
        }
        if !self.layer.allows(kind, target_layer) {
            return false;
        }
        if self.layer == Layer::Format && target_layer == Layer::Format {
            return target != ARCHIVE && self.builds_on.iter().any(|name| name == target);
        }
        true
    }

    fn depends_on(&self, target: &str) -> bool {
        self.dependencies.iter().any(|(name, _)| name == target)
    }
}

/// Returns one message per workspace dependency that points against the layer direction.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    Ok(violations(&workspace_packages(root)?))
}

fn violations(packages: &[Package]) -> Vec<String> {
    let layers: HashMap<&str, Layer> = packages
        .iter()
        .map(|package| (package.name.as_str(), package.layer))
        .collect();
    let mut violations = Vec::new();
    for package in packages {
        for (dependency, kind) in &package.dependencies {
            let Some(&target) = layers.get(dependency.as_str()) else {
                continue;
            };
            if !package.may_depend_on(*kind, dependency, target) {
                violations.push(format!(
                    "layer violation: {} ({:?}) has a {kind:?} dependency on {dependency} ({target:?}); \
                     a format crate names each format crate it builds on in \
                     [package.metadata.p4kx] builds-on, never {ARCHIVE}",
                    package.name, package.layer
                ));
            }
        }
        for declared in &package.builds_on {
            if package.layer != Layer::Format {
                violations.push(format!(
                    "layer violation: {} ({:?}) declares builds-on, which only format crates may",
                    package.name, package.layer
                ));
            } else if !package.depends_on(declared) {
                violations.push(format!(
                    "layer violation: {} declares builds-on {declared}, which is not a dependency",
                    package.name
                ));
            }
        }
    }
    violations
}

fn workspace_packages(root: &Path) -> Result<Vec<Package>> {
    crate::metadata::packages(root)?
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
    let builds_on = builds_on(&name, value.pointer("/metadata/p4kx/builds-on"))?;
    let dependencies = value
        .get("dependencies")
        .and_then(Value::as_array)
        .context("package without dependencies")?
        .iter()
        .filter_map(|dependency| {
            let name = dependency.get("name").and_then(Value::as_str)?.to_owned();
            let kind = if dependency.get("kind").and_then(Value::as_str) == Some("dev") {
                DependencyKind::Dev
            } else {
                DependencyKind::Normal
            };
            Some((name, kind))
        })
        .collect();
    Ok(Package {
        name,
        layer,
        builds_on,
        dependencies,
    })
}

fn builds_on(name: &str, value: Option<&Value>) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let Some(entries) = value.as_array() else {
        bail!("{name} has a builds-on that is not a list");
    };
    entries
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .with_context(|| format!("{name} has a builds-on entry that is not a crate name"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{DependencyKind, Layer, Package, violations};

    fn package(name: &str, layer: Layer, builds_on: &[&str], dependencies: &[&str]) -> Package {
        Package {
            name: name.to_owned(),
            layer,
            builds_on: builds_on.iter().map(|&entry| entry.to_owned()).collect(),
            dependencies: dependencies
                .iter()
                .map(|&entry| (entry.to_owned(), DependencyKind::Normal))
                .collect(),
        }
    }

    #[test]
    fn a_format_crate_may_depend_on_a_format_crate_it_builds_on() {
        let model = package(
            "p4kx-model",
            Layer::Format,
            &["p4kx-chunks"],
            &["p4kx-chunks"],
        );
        assert!(model.may_depend_on(DependencyKind::Normal, "p4kx-chunks", Layer::Format));
    }

    #[test]
    fn a_format_crate_may_not_depend_on_an_undeclared_format_crate() {
        let model = package(
            "p4kx-model",
            Layer::Format,
            &["p4kx-chunks"],
            &["p4kx-cryxml"],
        );
        assert!(!model.may_depend_on(DependencyKind::Normal, "p4kx-cryxml", Layer::Format));
        assert!(!model.may_depend_on(DependencyKind::Dev, "p4kx-cryxml", Layer::Format));
    }

    #[test]
    fn a_format_crate_may_never_depend_on_the_archive() {
        let model = package(
            "p4kx-model",
            Layer::Format,
            &["p4kx-archive"],
            &["p4kx-archive"],
        );
        assert!(!model.may_depend_on(DependencyKind::Normal, "p4kx-archive", Layer::Format));
    }

    #[test]
    fn a_format_crate_may_depend_on_the_foundation_and_dev_depend_on_the_testkit() {
        let chunks = package("p4kx-chunks", Layer::Format, &[], &[]);
        assert!(chunks.may_depend_on(DependencyKind::Normal, "p4kx-foundation", Layer::Foundation));
        assert!(chunks.may_depend_on(DependencyKind::Dev, "p4kx-testkit", Layer::Testkit));
        assert!(!chunks.may_depend_on(DependencyKind::Normal, "p4kx-testkit", Layer::Testkit));
    }

    #[test]
    fn a_crate_may_dev_depend_on_itself_to_turn_on_test_features() {
        let gui = package("p4kx-gui", Layer::Frontend, &[], &[]);
        assert!(gui.may_depend_on(DependencyKind::Dev, "p4kx-gui", Layer::Frontend));
        let chunks = package("p4kx-chunks", Layer::Format, &[], &[]);
        assert!(chunks.may_depend_on(DependencyKind::Dev, "p4kx-chunks", Layer::Format));
    }

    #[test]
    fn a_format_crate_may_not_depend_on_a_service_or_front_end() {
        let chunks = package("p4kx-chunks", Layer::Format, &["p4kx-gltf"], &[]);
        assert!(!chunks.may_depend_on(DependencyKind::Normal, "p4kx-gltf", Layer::Service));
        assert!(!chunks.may_depend_on(DependencyKind::Normal, "p4kx", Layer::Frontend));
    }

    #[test]
    fn a_service_crate_needs_no_builds_on_for_format_crates() {
        let gltf = package("p4kx-gltf", Layer::Service, &[], &[]);
        assert!(gltf.may_depend_on(DependencyKind::Normal, "p4kx-model", Layer::Format));
        assert!(gltf.may_depend_on(DependencyKind::Normal, "p4kx-archive", Layer::Format));
    }

    #[test]
    fn a_front_end_may_depend_only_on_services() {
        let cli = package("p4kx", Layer::Frontend, &[], &[]);
        assert!(cli.may_depend_on(DependencyKind::Normal, "p4kx-extraction", Layer::Service));
        assert!(!cli.may_depend_on(DependencyKind::Normal, "p4kx-model", Layer::Format));
    }

    #[test]
    fn violations_name_undeclared_edges_and_stale_or_misplaced_builds_on() {
        let packages = [
            package("p4kx-foundation", Layer::Foundation, &[], &[]),
            package("p4kx-chunks", Layer::Format, &[], &["p4kx-foundation"]),
            package("p4kx-cryxml", Layer::Format, &[], &["p4kx-foundation"]),
            package(
                "p4kx-model",
                Layer::Format,
                &["p4kx-chunks", "p4kx-cryxml"],
                &["p4kx-chunks", "p4kx-foundation"],
            ),
            package("p4kx-character", Layer::Format, &[], &["p4kx-chunks"]),
            package(
                "p4kx-gltf",
                Layer::Service,
                &["p4kx-model"],
                &["p4kx-model"],
            ),
        ];
        let found = violations(&packages);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert!(
            found.iter().any(|line| line.contains("p4kx-character")
                && line.contains("dependency on p4kx-chunks"))
        );
        assert!(
            found
                .iter()
                .any(|line| line.contains("builds-on p4kx-cryxml"))
        );
        assert!(
            found
                .iter()
                .any(|line| line.contains("p4kx-gltf (Service) declares builds-on"))
        );
    }

    #[test]
    fn the_declared_workspace_has_no_violations() {
        let packages = [
            package("p4kx-foundation", Layer::Foundation, &[], &[]),
            package("p4kx-chunks", Layer::Format, &[], &["p4kx-foundation"]),
            package(
                "p4kx-model",
                Layer::Format,
                &["p4kx-chunks"],
                &["p4kx-chunks", "p4kx-foundation"],
            ),
        ];
        assert_eq!(violations(&packages), Vec::<String>::new());
    }
}
