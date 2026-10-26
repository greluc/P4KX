// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::Dna;

use super::{Expressions, Look, Wearable};

/// Where the geometry of a [`Piece`] comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    /// An Entity; its geometry is chosen by the body variant.
    Wearable(Box<Wearable>),
    /// A skin file, wherever it comes from.
    File {
        path: String,
        material: Option<String>,
    },
    /// Something that could not be turned into a piece, with the reason.
    Failed(String),
}

/// Something to list about a piece that does not keep it from being worn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Note {
    /// The Character's base material is in no lookup table.
    UnknownMaterial(String),
    /// Parts of the Character's material choice that are not drawn.
    LookIgnored(String),
    /// The Loadout of the piece's Entity could not be read in full.
    LoadoutUnreadable(String),
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
    pub(crate) look: Option<Look>,
    pub(crate) material: Option<String>,
    pub(crate) follows_head: bool,
    pub(crate) notes: Vec<Note>,
    pub(crate) selecting: Vec<String>,
    pub(crate) variant_tags: Vec<String>,
    pub(crate) bespoke: Option<Bespoke>,
}

/// Where the skins of a story character's own head are, see [`Piece::bespoke`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Bespoke {
    pub(crate) folder: String,
    pub(crate) prefix: String,
    pub(crate) library: Option<String>,
}

impl Piece {
    const fn from_source(name: String, source: Source) -> Self {
        Self {
            name,
            source,
            shape: None,
            expressions: Expressions::None,
            look: None,
            material: None,
            follows_head: false,
            notes: Vec::new(),
            selecting: Vec::new(),
            variant_tags: Vec::new(),
            bespoke: None,
        }
    }

    /// Takes the sub-geometry of the Entity that is tagged with one of `flags`, the flags of the
    /// port the Entity sits at, before the sub-geometry of the body.
    #[must_use]
    pub fn selected_by(mut self, flags: Vec<String>) -> Self {
        self.selecting = flags;
        self
    }

    /// Chooses the Entity's material library by `tags`, see [`MaterialVariant::resolve`]; the
    /// body variant's tag is always among them.
    #[must_use]
    pub fn with_material_tags(mut self, tags: Vec<String>) -> Self {
        self.variant_tags = tags;
        self
    }

    /// Wears the skin a story character's own head has for this piece instead of the generic
    /// Protos skin its Record names: the file `<prefix>_<part>.skin` in `folder`, where `<part>`
    /// is what the generic file's name adds to its rig's name, e.g. `head` or `eyes`. A skin the
    /// Archive lacks leaves the Record's. A material library of the Record that the Archive lacks
    /// is replaced by `library`, the head's own, for such a skin.
    #[must_use]
    pub fn bespoke_in(
        mut self,
        folder: impl Into<String>,
        prefix: impl Into<String>,
        library: Option<String>,
    ) -> Self {
        self.bespoke = Some(Bespoke {
            folder: folder.into(),
            prefix: prefix.into(),
            library,
        });
        self
    }

    /// Wears the Entity `wearable`; the piece is named after its class name.
    #[must_use]
    pub fn wearable(wearable: Wearable) -> Self {
        Self::from_source(
            wearable.class_name().to_owned(),
            Source::Wearable(Box::new(wearable)),
        )
    }

    /// Wears the skin at `path`, a path below `Data`, in the node `name`, drawn with the
    /// material library `material` when given.
    #[must_use]
    pub fn file(
        name: impl Into<String>,
        path: impl Into<String>,
        material: Option<String>,
    ) -> Self {
        Self::from_source(
            name.into(),
            Source::File {
                path: path.into(),
                material,
            },
        )
    }

    /// Stands for a piece `name` that could not be found or read, because of `reason`.
    #[must_use]
    pub fn failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::from_source(name.into(), Source::Failed(reason.into()))
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

    /// Draws the piece with the material library at `path`, a path below `Data`, instead of the
    /// one its Record or its model names.
    #[must_use]
    pub fn with_material(mut self, path: impl Into<String>) -> Self {
        self.material = Some(path.into());
        self
    }

    /// Applies the material choices of a Character to the piece's materials, see [`Look`].
    #[must_use]
    pub fn looking(mut self, look: Look) -> Self {
        self.look = Some(look);
        self
    }

    /// Makes the piece follow the shape of the head piece named `head`: its vertices move as the
    /// vertices of the shaped head around them did, and the first mesh takes a morph target for
    /// each expression target of the head that moves it. For hair, brows, lashes and beards.
    #[must_use]
    pub const fn following_head(mut self) -> Self {
        self.follows_head = true;
        self
    }

    pub(crate) fn noting(mut self, note: Note) -> Self {
        self.notes.push(note);
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
