// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::Guid;

use crate::{Attachment, EntityNode, PortNode, Target};

/// One port of the default Loadout that holds something, at its path of port names.
#[derive(Debug, Clone)]
pub struct LoadoutEntry<'n> {
    pub(crate) path: String,
    pub(crate) port: &'n PortNode,
}

impl<'n> LoadoutEntry<'n> {
    /// Returns the path of port names joined by `/`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the port.
    #[must_use]
    pub const fn port(&self) -> &'n PortNode {
        self.port
    }

    /// Returns the Entity attached to the port, or `None` when the attachment cannot be resolved.
    #[must_use]
    pub fn entity(&self) -> Option<&'n EntityNode> {
        match &self.port.attachment {
            Attachment::Entity(node) => Some(node.as_ref()),
            _ => None,
        }
    }

    /// Returns the class name of the attached Entity, or the class name the Loadout names when
    /// the attachment cannot be resolved by it.
    #[must_use]
    pub fn class_name(&self) -> Option<&'n str> {
        match &self.port.attachment {
            Attachment::Entity(node) => Some(node.class_name()),
            Attachment::Unresolved(unresolved) => match &unresolved.target {
                Target::ClassName(name) => Some(name),
                _ => None,
            },
            Attachment::Empty => None,
        }
    }

    /// Returns the GUID of the attached Entity's Record, or of the Record the Loadout names when
    /// the attachment cannot be resolved.
    #[must_use]
    pub const fn guid(&self) -> Option<Guid> {
        match &self.port.attachment {
            Attachment::Entity(node) => Some(node.guid),
            Attachment::Unresolved(unresolved) => match &unresolved.target {
                Target::Reference(guid) => Some(*guid),
                _ => None,
            },
            Attachment::Empty => None,
        }
    }
}
