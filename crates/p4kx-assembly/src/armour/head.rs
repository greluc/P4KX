// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::Character;
use thiserror::Error;

use super::{BodyVariant, Expressions, Piece};

/// The rigs of the heads: the body variant, the CRC-32C of the rig's name, which is the
/// `gender_hash` of the DNA of the Characters made for it, and the name.
const RIGS: [(BodyVariant, u32, &str); 2] = [
    (
        BodyVariant::Male,
        0xdd6c_67f6,
        "protos_human_male_face_t1_pu",
    ),
    (
        BodyVariant::Female,
        0x9ef4_eb54,
        "protos_human_female_face_t1_pu",
    ),
];

/// Why the head of a Character cannot be made.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum HeadError {
    /// The Character holds no face DNA, so it has no head to shape.
    #[error("the character holds no face DNA")]
    NoFace,
    /// The DNA is made for a head rig this tool does not know.
    #[error("the DNA belongs to the unknown head rig {gender_hash:#010x}")]
    UnknownRig {
        /// The `gender_hash` of the DNA.
        gender_hash: u32,
    },
}

/// The head of a Character: the skin of the head rig that fits the Character's body, shaped by the
/// Character's face DNA.
///
/// Wear it with [`Armour::build`](super::Armour::build) on the [`BodyVariant`] it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    variant: BodyVariant,
    rig: &'static str,
    pieces: Vec<Piece>,
}

impl Head {
    /// Makes the head of `character`: the body variant and the skin, shaped by the Character's
    /// DNA, of the head rig the DNA's `gender_hash` names.
    ///
    /// # Errors
    ///
    /// Returns [`HeadError::NoFace`] for a Character without DNA and [`HeadError::UnknownRig`]
    /// when the DNA names a rig that is neither the male nor the female head.
    pub fn from_character(character: &Character) -> Result<Self, HeadError> {
        let dna = character.dna.as_ref().ok_or(HeadError::NoFace)?;
        let (variant, _, rig) = RIGS
            .iter()
            .find(|(_, hash, _)| *hash == dna.gender_hash)
            .ok_or(HeadError::UnknownRig {
                gender_hash: dna.gender_hash,
            })?;
        let folder = format!("Objects/Characters/Human/heads/{}/pu/{rig}", variant.code());
        let head =
            Piece::file("head", format!("{folder}/{rig}_head.skin"), None).shaped(dna.clone());
        Ok(Self {
            variant: *variant,
            rig,
            pieces: vec![head],
        })
    }

    /// Returns the head with the expression blendshapes `expressions` asks for written as morph
    /// targets of its mesh, see [`Piece::with_expressions`].
    #[must_use]
    pub fn with_expressions(mut self, expressions: &Expressions) -> Self {
        self.pieces = self
            .pieces
            .into_iter()
            .map(|piece| piece.with_expressions(expressions.clone()))
            .collect();
        self
    }

    /// Returns the body variant the head belongs on.
    #[must_use]
    pub const fn variant(&self) -> BodyVariant {
        self.variant
    }

    /// Returns the name of the head rig, e.g. `protos_human_male_face_t1_pu`.
    #[must_use]
    pub const fn rig(&self) -> &'static str {
        self.rig
    }

    /// Returns the pieces to wear on the body: the shaped head.
    #[must_use]
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }
}
