//! Reads and writes the CHF character files of Star Citizen: the fixed-size container with its
//! magic, CRC-32C and zstd-compressed body, the body with its inner versions, body type, face DNA,
//! item port tree and material choices, and a JSON form that converts back byte-exactly.

mod body_reader;
mod body_writer;
mod character;
mod chf;
mod container;
mod dna;
mod error;
mod guid;
mod item_port;
mod json_values;
mod material;

pub use character::{Character, FEMALE_VERSIONS, MALE_VERSIONS, Versions};
pub use chf::{Chf, ChfContainer};
pub use container::{CHF_MAGIC, CHF_SIZE, MAX_BODY_LEN};
pub use dna::{DNA_MAGIC, Dna, DnaBlend, face_part_name};
pub use error::ChfError;
pub use guid::{Guid, InvalidGuid};
pub use item_port::{ItemPort, MAX_PORT_DEPTH};
pub use material::{ColorParam, FloatParam, Material, SubMaterial, Texture};
