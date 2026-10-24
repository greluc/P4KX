// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Assembly service of P4KX: resolves the Loadout tree of an Entity and reads the CHF
//! files of Characters.
//!
//! [`Entities`] finds an Entity by record path, GUID or class name. [`LoadoutResolver`] resolves
//! its default Loadout (`SEntityComponentDefaultLoadoutParams`) into an [`EntityNode`] tree: every
//! port with the Entity attached to it, the port's definition (`SItemPortDef`: sizes, types and
//! transform), and recursively the ports of each attached Entity. A Loadout entry's own Loadout
//! overrides the attached Entity's default Loadout per port; ports a Loadout does not name fall
//! back to the port definition's default item. XML Loadouts are read through [`LoadoutFiles`],
//! e.g. [`ArchiveFiles`].
//!
//! A port that cannot be resolved, such as a dangling Reference, an unknown class name or a
//! cycle, is kept in the tree as [`Attachment::Unresolved`] with a [`LoadoutError`]; the rest of
//! the Loadout still resolves.
//!
//! [`LoadoutResolver::resolve_with`] applies [`LoadoutChanges`] while it resolves: ports left out
//! and Entities attached instead, each checked against the port's [`PortConstraints`] with the
//! [`ItemFacts`] of the Entity, reported as [`ChangeError`]s.
//!
//! [`Assembly::build`] turns the tree into a scene graph of placed models: the geometry each
//! Entity's Record names ([`Geometry`]), read through a [`ModelProvider`] such as
//! [`ArchiveModels`], placed at its port's helper in the parent's model, with the material
//! library the Entity chooses; [`Assembly::write_glb`] exports it as one GLB with a node per part
//! named `<port>/<entity>`. Parts that cannot be resolved are listed as [`Issue`]s.
//!
//! An Entity whose Record names a vehicle implementation (`VehicleComponentParams`) gets it with
//! the Modification it selects applied, as [`EntityNode::vehicle`]: the parts with their item
//! ports, treads and wheels, and per port the [`PortVisibility`] that [`PortNode::visibility`]
//! shows.
//!
//! [`audio`] indexes the audio of an Archive once: the ATL definitions that map audio trigger
//! names to Wwise events and the soundbanks whose `HIRC` sections hold those events, matched by
//! the events' Wwise short IDs ([`audio::wwise_id`]). [`audio::AudioIndex::entity_audio`] lists
//! the audio triggers an Entity names with their events and soundbanks.
//!
//! [`character`] reads and writes the CHF files of Characters and their JSON form.
//!
//! [`Armour::build`] puts [`Piece`]s of armour and clothing on the default body of a
//! [`BodyVariant`]: the body's skeleton and skins from its character definition, and per piece
//! the geometry its Record names for the variant's tag ([`Wearable`]), skinned to the body's
//! skeleton by bone name; [`Armour::write_glb`] exports it as one skinned GLB. A piece that cannot
//! be fitted is an [`Issue`].

mod armour;
mod assembly;
pub mod audio;
mod build;
mod changes;
mod compat;
mod entities;
mod error;
mod files;
mod flatten;
mod geometry;
mod issue;
mod markup;
mod merge;
mod node;
mod placement;
mod port;
mod provider;
mod read;
mod resolver;
mod spec;
mod tree;
mod vehicle;
mod walk;
mod xml;

pub use armour::{Armour, ArmourPart, BodyVariant, PartOrigin, Piece, Wearable};
pub use assembly::{
    Assembly, AssemblyOptions, MAX_ASSEMBLY_DEPTH, MAX_ASSEMBLY_PARTS, MAX_ASSEMBLY_VERTICES, Part,
    Placement,
};
pub use changes::{ChangeError, LoadoutChanges};
pub use compat::{AcceptedType, Incompatibility, ItemFacts, PortConstraints};
pub use entities::{ENTITY_TYPE, Entities};
pub use error::{AssemblyError, EntityLookupError, LoadoutError};
pub use files::{ArchiveFiles, LoadoutFileError, LoadoutFiles, MAX_XML_LOADOUT_LEN};
pub use geometry::{Geometry, GeometryVariant};
pub use issue::{Issue, IssueKind};
pub use p4kx_character as character;
pub use port::{PortDefinition, PortTransform, PortType};
pub use provider::{ArchiveModels, ModelFailure, ModelProvider, ProvidedModel};
pub use resolver::LoadoutResolver;
pub use tree::{Attachment, EntityNode, LoadoutEntry, PortNode, PortSource, Target, Unresolved};
pub use vehicle::{
    LandingGear, ModificationReport, OriginTally, PartKind, PartThreshold, PortVisibility,
    UnreadableValue, ValueOrigin, VehicleImplementation, VehicleItemPort, VehiclePart,
    VehicleTotals,
};
pub use xml::MAX_XML_INCLUDE_DEPTH;

/// The deepest element nesting of a vehicle implementation or patch file that is read; deeper
/// files are refused.
pub const MAX_XML_ELEMENT_DEPTH: usize = markup::MAX_ELEMENT_DEPTH;

/// The deepest chain of attached Entities below the root that is resolved; deeper attachments
/// are [`LoadoutError::TooDeep`].
pub const MAX_LOADOUT_DEPTH: usize = 32;

/// The most ports one Loadout tree holds; an Entity whose ports would pass it keeps the ports so
/// far and has [`LoadoutError::TooManyPorts`] among its issues.
pub const MAX_LOADOUT_PORTS: usize = 1 << 16;
