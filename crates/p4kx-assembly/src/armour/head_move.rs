// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_gltf::MorphTarget;
use p4kx_model::{Mesh, Model};

use super::face_build::Face;
use super::follow::follow;
use super::follow_morphs::{Followed, follow_morphs};

/// The head as it was made and as it was shaped, with its morph targets, which what follows the
/// head moves with.
pub(super) struct HeadMove {
    pub(super) before: Arc<Model>,
    pub(super) after: Arc<Model>,
    pub(super) morphs: Vec<MorphTarget>,
}

/// A job as it is worn: its model, its morph targets, the face of a head, and, for what follows
/// the head, the target of the head each of its morph targets stands for.
pub(super) struct Worn {
    pub(super) model: Arc<Model>,
    pub(super) morphs: Vec<MorphTarget>,
    pub(super) face: Option<Face>,
    pub(super) follows: Vec<usize>,
}

fn first_mesh(model: &Model) -> Option<&Mesh> {
    model.lods.first().and_then(|lod| lod.meshes.first())
}

/// Returns `model` with its vertices following the head as it was shaped, and with morph targets
/// that follow the expression targets of the head.
pub(super) fn followed(model: &Arc<Model>, head: &HeadMove) -> Worn {
    let (model, moved) = match (first_mesh(&head.before), first_mesh(&head.after)) {
        (Some(before), Some(after)) => (
            Arc::new(follow(model, before, after)),
            follow_morphs(model, before, &head.morphs),
        ),
        _ => (Arc::clone(model), Followed::none()),
    };
    Worn {
        model,
        morphs: moved.targets,
        face: None,
        follows: moved.of,
    }
}
