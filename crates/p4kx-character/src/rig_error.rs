// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

/// A failure to read a DNA rig file or to apply a Character's DNA to it.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RigError {
    /// The file is shorter than the header and pointer table.
    #[error("the DNA rig is truncated: {have} bytes, the header and pointer table need {need}")]
    HeaderTruncated {
        /// The bytes the header and pointer table need.
        need: usize,
        /// The bytes the input has.
        have: usize,
    },
    /// The file does not start with the DNA rig magic.
    #[error("not a DNA rig file: the magic is not `DNA V1.6`")]
    BadMagic,
    /// The header states no variants.
    #[error("the DNA rig states no variants")]
    ZeroVariants,
    /// The rig has more heads and regions than this crate blends.
    #[error("the rig has {variants} heads and {regions} regions, more than {max} cells")]
    TooLarge {
        /// The number of heads.
        variants: usize,
        /// The number of regions.
        regions: usize,
        /// The most cells (heads times regions) this crate blends.
        max: usize,
    },
    /// A pointer table slot points outside the payload.
    #[error("slot {slot} points to offset {offset}, outside the payload of {file_len} bytes")]
    SlotOutOfRange {
        /// The slot.
        slot: usize,
        /// The offset it holds.
        offset: u64,
        /// The size of the file.
        file_len: usize,
    },
    /// A slot the requested data lives in is not set.
    #[error("the DNA rig has no {what} (slot {slot} is empty)")]
    SlotUnset {
        /// What the slot holds.
        what: &'static str,
        /// The slot.
        slot: usize,
    },
    /// A slot is shorter than its contents need.
    #[error("the {what} (slot {slot}) holds {have} bytes, {need} are needed")]
    SlotTooShort {
        /// What the slot holds.
        what: &'static str,
        /// The slot.
        slot: usize,
        /// The bytes the contents need.
        need: u64,
        /// The bytes the slot has.
        have: usize,
    },
    /// The contents of a slot break the layout.
    #[error("the {what} (slot {slot}) is malformed: {reason}")]
    Malformed {
        /// What the slot holds.
        what: &'static str,
        /// The slot.
        slot: usize,
        /// What is wrong.
        reason: &'static str,
    },
    /// The mesh does not have the vertex count the rig was made for.
    #[error("the rig covers {rig} vertices, the mesh has {mesh}")]
    VertexCount {
        /// The vertices the rig covers.
        rig: usize,
        /// The vertices the mesh has.
        mesh: usize,
    },
    /// The Character's DNA names a head the rig does not have.
    #[error("the DNA names head {head_id}, the rig has {variants} heads")]
    HeadOutOfRange {
        /// The head ID the DNA names.
        head_id: u16,
        /// The heads the rig has.
        variants: usize,
    },
    /// The Character's DNA has more face parts than the rig has regions.
    #[error("the DNA has {parts} face parts, the rig has {regions} regions")]
    FacePartCount {
        /// The face parts the DNA has.
        parts: usize,
        /// The regions the rig has.
        regions: usize,
    },
    /// A matrix does not have the shape of the rig.
    #[error(
        "the DNA matrix is {matrix_variants} x {matrix_regions}, the rig is {variants} x {regions}"
    )]
    MatrixShape {
        /// The variants of the matrix.
        matrix_variants: usize,
        /// The regions of the matrix.
        matrix_regions: usize,
        /// The variants of the rig.
        variants: usize,
        /// The regions of the rig.
        regions: usize,
    },
}
