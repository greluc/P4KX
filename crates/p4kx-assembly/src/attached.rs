// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_model::{MaterialStatus, Model, Transform};

/// A rigid model a part carries at a bone of its skeleton: a bone attachment of the character
/// definition the part's geometry is, e.g. the housing of a thruster.
#[derive(Debug, Clone)]
pub struct AttachedModel {
    pub(crate) name: String,
    pub(crate) bone: String,
    pub(crate) local: Transform,
    pub(crate) found: bool,
    pub(crate) entry: String,
    pub(crate) model: Arc<Model>,
    pub(crate) materials: Arc<MaterialStatus>,
}

impl AttachedModel {
    /// Returns the attachment's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the bone of the part's skeleton the model is bound to.
    #[must_use]
    pub fn bone(&self) -> &str {
        &self.bone
    }

    /// Returns whether the part's skeleton has the bone; when not, the model sits at the part's
    /// origin, offset by the attachment's own position and rotation.
    #[must_use]
    pub const fn is_bone_found(&self) -> bool {
        self.found
    }

    /// Returns the transform from the model's space to the part's, in `CryEngine` coordinates, Z
    /// up and in metres: the bone in bind pose, offset by the attachment.
    #[must_use]
    pub const fn local(&self) -> &Transform {
        &self.local
    }

    /// Returns the path of the model's file.
    #[must_use]
    pub fn geometry(&self) -> &str {
        &self.entry
    }

    /// Returns the model.
    #[must_use]
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Returns the material library the model is drawn with.
    #[must_use]
    pub fn materials(&self) -> &MaterialStatus {
        &self.materials
    }
}
