// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The 3D viewer of the Model tab: the model flattened into a [`Scene`], the orbit [`Camera`]
//! that looks at it, the [`Playback`] of its skeleton and animations, and the wgpu drawing of it.

mod assembly;
mod bounds;
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

pub(crate) use assembly::hidden_mask;
pub use assembly::{
    AssemblyScene, BuildError, BuildOptions, DrawList, GroupDraw, Hooks, Instance, Left,
    MAX_VIEW_ITEMS, MAX_VIEW_TEXTURE_BYTES, MAX_VIEW_TRIANGLES, MAX_VIEW_VERTICES, MIN_PART_PIXELS,
    PartInfo, Progress, VIEW_TEXTURE_MIP,
};
pub use camera::{Camera, Matrix, Step, Uniforms};
pub(crate) use gpu::{AssemblyDraw, Look, Subject, paint_callback, release_callback};
pub use gpu::{
    GpuReport, MAX_TEXTURE_SIDE, Rejected, Unavailable, check_buffer, check_texture, install,
    target_extent,
};
pub use playback::{Control, Playback};
pub use preview::{MODEL_FILE_LIMIT, ModelPreview, ModelStatus, is_model_path};
pub use scene::{
    MorphSpan,
    Bounds, MAX_SCENE_TRIANGLES, MAX_SCENE_VERTICES, SKIN_STRIDE, Scene, SceneError, SkinVertex,
    Vertex,
};
pub use skeleton::{MAX_SKELETON_BONES, Pose, Skeleton};
pub use support::{Problem, ViewerSupport};
