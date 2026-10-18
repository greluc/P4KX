//! Assembly service of P4K-Extractor: resolves the Loadout tree of an Entity and reads the CHF
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
//! [`character`] reads and writes the CHF files of Characters and their JSON form.

mod entities;
mod error;
mod files;
mod merge;
mod node;
mod port;
mod read;
mod resolver;
mod spec;
mod tree;
mod xml;

pub use entities::{ENTITY_TYPE, Entities};
pub use error::{EntityLookupError, LoadoutError};
pub use files::{ArchiveFiles, LoadoutFileError, LoadoutFiles, MAX_XML_LOADOUT_LEN};
pub use p4kx_character as character;
pub use port::{PortDefinition, PortTransform, PortType};
pub use resolver::LoadoutResolver;
pub use tree::{Attachment, EntityNode, PortNode, PortSource, Target, Unresolved};
pub use xml::MAX_XML_INCLUDE_DEPTH;

/// The deepest chain of attached Entities below the root that is resolved; deeper attachments
/// are [`LoadoutError::TooDeep`].
pub const MAX_LOADOUT_DEPTH: usize = 32;

/// The most ports one Loadout tree holds; an Entity whose ports would pass it keeps the ports so
/// far and has [`LoadoutError::TooManyPorts`] among its issues.
pub const MAX_LOADOUT_PORTS: usize = 1 << 16;
