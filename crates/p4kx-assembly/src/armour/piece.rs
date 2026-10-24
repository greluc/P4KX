// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::Dna;

use super::{Expressions, Wearable};

/// Where the geometry of a [`Piece`] comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    /// An Entity; its geometry is chosen by the body variant.
    Wearable(Wearable),
    /// A skin file, wherever it comes from.
    File {
        path: String,
        material: Option<String>,
    },
    /// Something that could not be turned into a piece, with the reason.
    Failed(String),
}

/// One piece worn on a body: a piece of armour or clothing, or any skin that belongs on the
/// skeleton.
///
/// A piece comes from an Entity ([`Piece::wearable`]) or from a file ([`Piece::file`]), so that
/// pieces from a Loadout or a head from a character file are built the same way. One that could
/// not be found is kept as [`Piece::failed`], so that the report lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub(crate) name: String,
    pub(crate) source: Source,
    pub(crate) shape: Option<Dna>,
    pub(crate) expressions: Expressions,
}

impl Piece {
    /// Wears the Entity `wearable`; the piece is named after its class name.
    #[must_use]
    pub fn wearable(wearable: Wearable) -> Self {
        Self {
            name: wearable.class_name().to_owned(),
            source: Source::Wearable(wearable),
            shape: None,
            expressions: Expressions::None,
        }
    }

    /// Wears the skin at `path`, a path below `Data`, in the node `name`, drawn with the
    /// material library `material` when given.
    #[must_use]
    pub fn file(
        name: impl Into<String>,
        path: impl Into<String>,
        material: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: Source::File {
                path: path.into(),
                material,
            },
            shape: None,
            expressions: Expressions::None,
        }
    }

    /// Stands for a piece `name` that could not be found or read, because of `reason`.
    #[must_use]
    pub fn failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: Source::Failed(reason.into()),
            shape: None,
            expressions: Expressions::None,
        }
    }

    /// Shapes the piece by `dna`, the face DNA of a Character: the vertices of the piece's model
    /// are moved by the neutral morphs of the DNA rig that sits beside the model, see
    /// [`DnaRig`](p4kx_character::DnaRig). A piece whose rig is missing or does not fit its model
    /// is left out and listed as [`IssueKind::ShapeFailed`](crate::IssueKind::ShapeFailed).
    #[must_use]
    pub fn shaped(mut self, dna: Dna) -> Self {
        self.shape = Some(dna);
        self
    }

    /// Writes the expression blendshapes `expressions` names as morph targets of the piece's mesh,
    /// mixed by the DNA the piece is shaped by, see [`Piece::shaped`], so that they move the
    /// Character's own neutral face. A piece that is not shaped has none. The targets are named
    /// by the `RigLogic` chunk of the skin; if they cannot be made the piece is kept without them
    /// and listed as [`IssueKind::MorphsFailed`](crate::IssueKind::MorphsFailed), and a name no
    /// blendshape has as [`IssueKind::UnknownExpression`](crate::IssueKind::UnknownExpression).
    #[must_use]
    pub fn with_expressions(mut self, expressions: Expressions) -> Self {
        self.expressions = expressions;
        self
    }

    /// Renames the piece's node, e.g. to `<port>/<entity>`.
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Returns the name of the piece's node.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
