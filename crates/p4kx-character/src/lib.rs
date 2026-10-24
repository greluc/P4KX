// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reads and writes the CHF character files of Star Citizen: the fixed-size container with its
//! magic, CRC-32C and zstd-compressed body, the body with its inner versions, body type, face DNA,
//! item port tree and material choices, and a JSON form that converts back byte-exactly.
//!
//! Reads the DNA rigs (`.dna` files) the face DNA of a Character is applied to: [`DnaRig`] gives
//! the names, joint offsets, neutral and expression morphs, joint and region weights and delta
//! records of the heads of a rig, and [`DnaMatrix`] the weights with which a Character's DNA
//! blends them. [`ExpressionBlend`] turns the expression morphs of the stock heads into the
//! blendshapes of a Character's head, and [`RigLogic`] names them.

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
mod names;
mod rig;
mod rig_blend;
mod rig_bytes;
mod rig_deltas;
mod rig_error;
mod rig_expression;
mod rig_logic;
mod rig_matrix;
mod rig_neutral;
mod rig_tables;

pub use character::{Character, FEMALE_VERSIONS, MALE_VERSIONS, Versions};
pub use chf::{Chf, ChfContainer};
pub use container::{CHF_MAGIC, CHF_SIZE, MAX_BODY_LEN};
pub use dna::{DNA_MAGIC, Dna, DnaBlend, MAX_FACE_PARTS, face_part_name};
pub use error::ChfError;
pub use guid::{Guid, InvalidGuid};
pub use item_port::{ItemPort, MAX_PORT_DEPTH};
pub use material::{ColorParam, FloatParam, Material, SubMaterial, Texture};
pub use names::{
    BODY_COLOR, EYE_COLOR, HAIR_DYE_COLOR, HAIR_TINT_COLOR, lowered_hash, name_hash,
    skin_attachment,
};
pub use rig::{DnaRig, RIG_MAGIC, RigHeader};
pub use rig_blend::{ExpressionBlend, ExpressionTarget};
pub use rig_deltas::{
    DeltaEntry, DeltaRecord, DeltaRecords, DeltaSet, VariantDeltas, VariantHeader,
};
pub use rig_error::RigError;
pub use rig_expression::{ExpressionCell, ExpressionMorphs};
pub use rig_logic::{RIG_LOGIC_MAGIC, RigLogic, RigLogicError};
pub use rig_matrix::{DnaMatrix, MAX_RIG_CELLS};
pub use rig_neutral::NeutralMorphs;
pub use rig_tables::{JointWeights, RegionMasks, VariantJoints};
