// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

use crate::LoadoutError;

/// Something an Assembly could not do for a part, found while building it.
///
/// A part that cannot be resolved is left out or kept without geometry, and the rest of the
/// Assembly is built; the issue says which part and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) kind: IssueKind,
}

impl Issue {
    /// Returns the path of the part's port from the root, port names joined by `/`; empty for
    /// the root Entity.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the class name of the part's Entity, or what the Loadout names for an attachment
    /// that cannot be resolved.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns what went wrong.
    #[must_use]
    pub const fn kind(&self) -> &IssueKind {
        &self.kind
    }
}

/// What went wrong with a part of an Assembly.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum IssueKind {
    /// The Loadout names an Entity that cannot be resolved, so the port holds no part.
    #[error("the attachment cannot be resolved: {0}")]
    Unresolved(LoadoutError),
    /// The Entity's Record names no geometry, so the part has no model.
    #[error("the Entity names no geometry")]
    NoGeometry,
    /// The geometry file the Record names does not exist, so the part has no model.
    #[error("the geometry {path} does not exist")]
    GeometryMissing {
        /// The path the Record names.
        path: String,
    },
    /// The geometry file cannot be read as a model, so the part has no model.
    #[error("the geometry {path} cannot be read: {reason}")]
    GeometryUnreadable {
        /// The path the Record names.
        path: String,
        /// What is wrong.
        reason: String,
    },
    /// The material library of the model is missing or unreadable, so the engine's default
    /// material is used.
    #[error("{reason}")]
    MaterialLibrary {
        /// What is wrong.
        reason: String,
    },
    /// The port's helper is no node of the parent's model, so the part sits at the parent's
    /// origin.
    #[error("the helper {helper} is no node of the parent's model")]
    HelperNotFound {
        /// The helper the port names.
        helper: String,
    },
    /// The Assembly holds [`MAX_ASSEMBLY_PARTS`](crate::MAX_ASSEMBLY_PARTS) parts, so further
    /// parts are left out.
    #[error("the Assembly reached {limit} parts; further parts are left out")]
    TooManyParts {
        /// The limit.
        limit: usize,
    },
    /// The parts' geometry reached [`MAX_ASSEMBLY_VERTICES`](crate::MAX_ASSEMBLY_VERTICES)
    /// vertices, so this part has no model.
    #[error("the Assembly reached {limit} vertices; the part's geometry is left out")]
    TooManyVertices {
        /// The limit.
        limit: usize,
    },
    /// The part is nested deeper than [`MAX_ASSEMBLY_DEPTH`](crate::MAX_ASSEMBLY_DEPTH), so it
    /// and what is below it are left out.
    #[error("the part is nested deeper than {limit} levels and left out")]
    TooDeep {
        /// The limit.
        limit: usize,
    },
}

impl IssueKind {
    /// Returns a stable code for the kind, e.g. `geometry_missing`, for scripts; the message is
    /// text for people.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Unresolved(_) => "unresolved",
            Self::NoGeometry => "no_geometry",
            Self::GeometryMissing { .. } => "geometry_missing",
            Self::GeometryUnreadable { .. } => "geometry_unreadable",
            Self::MaterialLibrary { .. } => "material_library",
            Self::HelperNotFound { .. } => "helper_not_found",
            Self::TooManyParts { .. } => "too_many_parts",
            Self::TooManyVertices { .. } => "too_many_vertices",
            Self::TooDeep { .. } => "too_deep",
        }
    }

    /// Returns whether the part, or its geometry, is left out of the Assembly because of this.
    #[must_use]
    pub const fn skips(&self) -> bool {
        matches!(
            self,
            Self::Unresolved(_)
                | Self::GeometryMissing { .. }
                | Self::GeometryUnreadable { .. }
                | Self::TooManyParts { .. }
                | Self::TooManyVertices { .. }
                | Self::TooDeep { .. }
        )
    }
}
