// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The 3D viewer of the Model tab: the model flattened into a [`Scene`], the orbit [`Camera`]
//! that looks at it, the [`Playback`] of its skeleton and animations, and the wgpu drawing of it.

mod camera;
#[cfg(test)]
mod fixtures;
mod gpu;
mod playback;
mod preview;
mod rigid;
mod scene;
mod skeleton;
mod support;

pub use camera::{Camera, Matrix, Step, Uniforms};
pub use gpu::{
    GpuReport, MAX_TEXTURE_SIDE, Rejected, Unavailable, check_buffer, check_texture, install,
    target_extent,
};
pub(crate) use gpu::{Look, paint_callback};
pub use playback::{Control, Playback};
pub use preview::{MODEL_FILE_LIMIT, ModelPreview, ModelStatus, is_model_path};
pub use scene::{
    Bounds, MAX_SCENE_TRIANGLES, MAX_SCENE_VERTICES, SKIN_STRIDE, Scene, SceneError, SkinVertex,
    Vertex,
};
pub use skeleton::{MAX_SKELETON_BONES, Pose, Skeleton};
pub use support::{Problem, ViewerSupport};
