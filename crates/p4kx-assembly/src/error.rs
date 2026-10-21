use p4kx_dataforge::{Guid, ValueError};
use thiserror::Error;

use crate::{MAX_LOADOUT_DEPTH, MAX_LOADOUT_PORTS};

/// Why a port of a Loadout, or the Loadout of an Entity, cannot be resolved.
///
/// It is kept in the tree at its place, so the rest of the Loadout still resolves.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum LoadoutError {
    /// A Reference names a Record that does not exist.
    #[error("the Reference refers to the Record {guid}, which does not exist")]
    DanglingReference {
        /// The GUID of the missing Record.
        guid: Guid,
    },
    /// A Reference cannot be read.
    #[error("the Reference cannot be read: {reason}")]
    InvalidReference {
        /// Why the value cannot be read.
        reason: ValueError,
    },
    /// No Entity has the class name.
    #[error("no Entity has the class name {name}")]
    UnknownClass {
        /// The class name.
        name: String,
    },
    /// Several Entities have the class name; the first in definition order is attached.
    #[error(
        "{} Entities have the class name {name}; the first is attached: {}",
        candidates.len(),
        candidates.join(", ")
    )]
    AmbiguousClass {
        /// The class name.
        name: String,
        /// The GUID and record path of each Entity, in definition order.
        candidates: Vec<String>,
    },
    /// The Record is no Entity.
    #[error("the Record {guid} is a {type_name}, not an Entity")]
    NotAnEntity {
        /// The GUID of the Record.
        guid: Guid,
        /// The Record's type.
        type_name: String,
    },
    /// The Record cannot be read.
    #[error("the Record {guid} cannot be read: {reason}")]
    UnreadableRecord {
        /// The GUID of the Record.
        guid: Guid,
        /// Why it cannot be read.
        reason: String,
    },
    /// The Entity is already attached further up the same branch.
    #[error(
        "the Entity {guid} is already attached further up this branch; attaching it again never ends"
    )]
    Cycle {
        /// The GUID of the Entity.
        guid: Guid,
    },
    /// The branch is deeper than [`MAX_LOADOUT_DEPTH`].
    #[error("the Loadout is nested deeper than {MAX_LOADOUT_DEPTH} Entities")]
    TooDeep,
    /// The tree reached [`MAX_LOADOUT_PORTS`] ports, so the Entity's further ports are left out.
    #[error("the Loadout tree reached {MAX_LOADOUT_PORTS} ports; further ports are left out")]
    TooManyPorts,
    /// An XML Loadout is named, but no Archive to read it from was given.
    #[error("the XML Loadout {path} is not read without its Archive")]
    NoLoadoutFiles {
        /// The path the Record names.
        path: String,
    },
    /// An XML Loadout cannot be read.
    #[error("the XML Loadout {path} cannot be read: {reason}")]
    XmlLoadout {
        /// The path the Record or including Loadout names.
        path: String,
        /// Why it cannot be read.
        reason: String,
    },
    /// The vehicle implementation of an Entity, or its Modification, cannot be read in full.
    #[error("the vehicle implementation {path} cannot be read: {reason}")]
    VehicleImplementation {
        /// The path the Record names, or of the file that fails.
        path: String,
        /// Why it cannot be read.
        reason: String,
    },
}

impl LoadoutError {
    /// Returns a stable code for the kind of error, e.g. `dangling_reference`, for scripts; the
    /// message is text for people.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::DanglingReference { .. } => "dangling_reference",
            Self::InvalidReference { .. } => "invalid_reference",
            Self::UnknownClass { .. } => "unknown_class",
            Self::AmbiguousClass { .. } => "ambiguous_class",
            Self::NotAnEntity { .. } => "not_an_entity",
            Self::UnreadableRecord { .. } => "unreadable_record",
            Self::Cycle { .. } => "cycle",
            Self::TooDeep => "too_deep",
            Self::TooManyPorts => "too_many_ports",
            Self::NoLoadoutFiles { .. } => "no_loadout_files",
            Self::XmlLoadout { .. } => "xml_loadout",
            Self::VehicleImplementation { .. } => "vehicle_implementation",
        }
    }
}

/// An Assembly cannot be built at all.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssemblyError {
    /// The source of the models fails for every file, e.g. the Archive key is missing; parts that
    /// merely cannot be resolved are issues of the Assembly instead.
    #[error("the models cannot be read")]
    Provider(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// An Entity named by record path, GUID or class name cannot be found.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum EntityLookupError {
    /// No Entity matches.
    #[error("no Entity has the record path, GUID or class name {text}")]
    NotFound {
        /// The text that was looked up.
        text: String,
    },
    /// Several Entities match.
    #[error("{text} names {} Entities: {}", candidates.len(), candidates.join(", "))]
    Ambiguous {
        /// The text that was looked up.
        text: String,
        /// The GUID and record path of each match, in definition order.
        candidates: Vec<String>,
    },
    /// The Record found is no Entity.
    #[error("the Record {text} is a {type_name}, not an Entity")]
    NotAnEntity {
        /// The text that was looked up.
        text: String,
        /// The Record's type.
        type_name: String,
    },
}
