use std::sync::Arc;

use p4kx_dataforge::Guid;

use crate::entities::class_name_of;
use crate::{Geometry, LoadoutError, PortDefinition};

/// One Entity in a Loadout tree: its Record and its ports with what they hold.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityNode {
    pub(crate) guid: Guid,
    pub(crate) record_path: String,
    pub(crate) name: String,
    pub(crate) ports: Vec<PortNode>,
    pub(crate) issues: Vec<LoadoutError>,
    pub(crate) geometry: Option<Geometry>,
}

impl EntityNode {
    /// Returns the geometry the Entity's Record names, or `None` when it names none.
    #[must_use]
    pub const fn geometry(&self) -> Option<&Geometry> {
        self.geometry.as_ref()
    }

    /// Returns the GUID of the Entity's Record.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the record path, without `.xml`.
    #[must_use]
    pub fn record_path(&self) -> &str {
        &self.record_path
    }

    /// Returns the Record name, e.g. `EntityClassDefinition.AEGS_Avenger_Titan`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the class name, the Record name after `EntityClassDefinition.`.
    #[must_use]
    pub fn class_name(&self) -> &str {
        class_name_of(&self.name).unwrap_or(&self.name)
    }

    /// Returns the ports: those the Loadouts name in Loadout order, then the defined ports no
    /// Loadout names in definition order.
    #[must_use]
    pub fn ports(&self) -> &[PortNode] {
        &self.ports
    }

    /// Returns why the Entity's own Loadout cannot be read in full, e.g. a missing XML Loadout.
    #[must_use]
    pub fn issues(&self) -> &[LoadoutError] {
        &self.issues
    }

    /// Returns every port in the tree below this Entity, depth first in port order, with its
    /// path of port names joined by `/`.
    #[must_use]
    pub fn walk(&self) -> Vec<(String, &PortNode)> {
        let mut ports = Vec::new();
        walk(self, "", &mut ports);
        ports
    }

    /// Returns every port in the tree that cannot be resolved, with its path of port names
    /// joined by `/`, depth first in port order.
    pub fn unresolved(&self) -> impl Iterator<Item = (String, &Unresolved)> {
        self.walk()
            .into_iter()
            .filter_map(|(path, port)| match &port.attachment {
                Attachment::Unresolved(unresolved) => Some((path, unresolved)),
                _ => None,
            })
    }
}

fn walk<'n>(node: &'n EntityNode, prefix: &str, ports: &mut Vec<(String, &'n PortNode)>) {
    walk_ports(&node.ports, prefix, ports);
}

fn walk_ports<'n>(list: &'n [PortNode], prefix: &str, ports: &mut Vec<(String, &'n PortNode)>) {
    for port in list {
        let path = if prefix.is_empty() {
            port.name.clone()
        } else {
            format!("{prefix}/{}", port.name)
        };
        ports.push((path.clone(), port));
        walk_ports(&port.nested, &path, ports);
        if let Attachment::Entity(child) = &port.attachment {
            walk(child, &path, ports);
        }
    }
}

/// One port of an Entity in a Loadout tree.
#[derive(Debug, Clone, PartialEq)]
pub struct PortNode {
    pub(crate) name: String,
    pub(crate) source: PortSource,
    pub(crate) definition: Option<Arc<PortDefinition>>,
    pub(crate) attachment: Attachment,
    pub(crate) nested: Vec<Self>,
}

impl PortNode {
    /// Returns the ports of the nested Loadout of an empty port: a Loadout entry that names no
    /// Entity but carries entries of its own, e.g. the gear below a character's `Body_ItemPort`.
    /// Their source is [`PortSource::Override`]; empty for every other port.
    #[must_use]
    pub fn nested(&self) -> &[Self] {
        &self.nested
    }

    /// Returns the port name, as the Loadout or the port definition spells it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns where the port's attachment comes from.
    #[must_use]
    pub const fn source(&self) -> PortSource {
        self.source
    }

    /// Returns the port's definition in the Entity Record, or `None` when the Record defines no
    /// port of this name, e.g. a ship port from its vehicle definition.
    #[must_use]
    pub fn definition(&self) -> Option<&PortDefinition> {
        self.definition.as_deref()
    }

    /// Returns what the port holds.
    #[must_use]
    pub const fn attachment(&self) -> &Attachment {
        &self.attachment
    }
}

/// Where the attachment of a port comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortSource {
    /// The Entity's own default Loadout.
    Loadout,
    /// The Loadout the entry attaching the Entity gives, which overrides the Entity's own.
    Override,
    /// The default item of the port definition, used when no Loadout names the port.
    PortDefault,
    /// Nothing names the port; it is defined but holds nothing.
    Unset,
}

impl PortSource {
    /// Returns the stable code of the source, e.g. `override`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Loadout => "loadout",
            Self::Override => "override",
            Self::PortDefault => "port_default",
            Self::Unset => "unset",
        }
    }
}

/// What a port holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Attachment {
    /// Nothing.
    Empty,
    /// An Entity with its own ports.
    Entity(Box<EntityNode>),
    /// An Entity that cannot be resolved, with the reason.
    Unresolved(Unresolved),
}

/// An attachment that cannot be resolved: what was named and why it fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    pub(crate) target: Target,
    pub(crate) error: LoadoutError,
}

impl Unresolved {
    /// Returns what the Loadout names.
    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }

    /// Returns why it cannot be resolved.
    #[must_use]
    pub const fn error(&self) -> &LoadoutError {
        &self.error
    }
}

/// What a Loadout names to attach to a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A Record by Reference.
    Reference(Guid),
    /// An Entity by class name.
    ClassName(String),
    /// A Reference that cannot be read.
    Unreadable,
}
