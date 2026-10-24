// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::Wearable;

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
}

impl Piece {
    /// Wears the Entity `wearable`; the piece is named after its class name.
    #[must_use]
    pub fn wearable(wearable: Wearable) -> Self {
        Self {
            name: wearable.class_name().to_owned(),
            source: Source::Wearable(wearable),
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
        }
    }

    /// Stands for a piece `name` that could not be found or read, because of `reason`.
    #[must_use]
    pub fn failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: Source::Failed(reason.into()),
        }
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
