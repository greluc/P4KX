// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{Attachment, EntityNode, PortConstraints, PortSource, Target};

use crate::model::describe;

/// What the changes do to a port of the Loadout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowChange {
    /// The port keeps what the default Loadout gives it.
    Kept,
    /// The port is left out with everything below it.
    Excluded,
    /// Another Entity is attached instead.
    Replaced,
}

/// One port of the Loadout tree as the picker shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutRow {
    path: String,
    port: String,
    depth: usize,
    entity: Option<String>,
    change: RowChange,
    unresolved: Option<String>,
    docked: bool,
    has_children: bool,
    constraints: Option<PortConstraints>,
}

impl LoadoutRow {
    /// Returns the path of port names from the Entity joined by `/`, the key of the changes.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the name of the port.
    #[must_use]
    pub fn port(&self) -> &str {
        &self.port
    }

    /// Returns how deep the port sits below the Entity: 0 for its own ports.
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the class name of the Entity the port holds, or the class name the Loadout names
    /// when it cannot be resolved, or `None` when the port holds nothing.
    #[must_use]
    pub fn entity(&self) -> Option<&str> {
        self.entity.as_deref()
    }

    /// Returns what the changes do to the port.
    #[must_use]
    pub const fn change(&self) -> RowChange {
        self.change
    }

    /// Returns why the attachment cannot be resolved, if it cannot.
    #[must_use]
    pub fn unresolved(&self) -> Option<&str> {
        self.unresolved.as_deref()
    }

    /// Returns whether the port holds a docked vehicle, whose own ports are not listed.
    #[must_use]
    pub const fn is_docked(&self) -> bool {
        self.docked
    }

    /// Returns whether ports sit below this one.
    #[must_use]
    pub const fn has_children(&self) -> bool {
        self.has_children
    }

    /// Returns what the port takes, or `None` when it states nothing and takes any Entity.
    #[must_use]
    pub const fn constraints(&self) -> Option<&PortConstraints> {
        self.constraints.as_ref()
    }
}

/// The ports of a resolved Loadout in tree order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutView {
    rows: Vec<LoadoutRow>,
}

impl LoadoutView {
    pub(super) fn of(tree: &EntityNode) -> Self {
        let ports = tree.walk();
        let depth = |path: &str| path.matches('/').count();
        let rows = ports
            .iter()
            .enumerate()
            .map(|(at, (path, port))| {
                let own = depth(path);
                let (entity, unresolved, docked) = match port.attachment() {
                    Attachment::Empty => (None, None, false),
                    Attachment::Entity(node) => (
                        Some(node.class_name().to_owned()),
                        None,
                        node.is_docked_vehicle(),
                    ),
                    Attachment::Unresolved(unresolved) => (
                        match unresolved.target() {
                            Target::ClassName(name) => Some(name.clone()),
                            _ => None,
                        },
                        Some(describe(unresolved.error())),
                        false,
                    ),
                };
                LoadoutRow {
                    path: path.clone(),
                    port: port.name().to_owned(),
                    depth: own,
                    entity,
                    change: match port.source() {
                        PortSource::Excluded => RowChange::Excluded,
                        PortSource::Replaced => RowChange::Replaced,
                        _ => RowChange::Kept,
                    },
                    unresolved,
                    docked,
                    has_children: ports.get(at + 1).is_some_and(|(next, _)| depth(next) > own),
                    constraints: port.constraints().cloned(),
                }
            })
            .collect();
        Self { rows }
    }

    /// Returns every port in tree order.
    #[must_use]
    pub fn rows(&self) -> &[LoadoutRow] {
        &self.rows
    }
}
