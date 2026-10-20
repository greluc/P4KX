//! The 3D viewer of the Model tab: the model flattened into a [`Scene`], the orbit [`Camera`]
//! that looks at it, and the wgpu drawing of it.

mod camera;
mod gpu;
mod preview;
mod scene;
mod support;

pub use camera::{Camera, Matrix, Step, Uniforms};
pub(crate) use gpu::paint_callback;
pub use gpu::{
    GpuReport, MAX_TEXTURE_SIDE, Rejected, Unavailable, check_buffer, check_texture, install,
    target_extent,
};
pub use preview::{MODEL_FILE_LIMIT, ModelPreview, ModelStatus, is_model_path};
pub use scene::{Bounds, MAX_SCENE_TRIANGLES, MAX_SCENE_VERTICES, Scene, SceneError, Vertex};
pub use support::{Problem, ViewerSupport};
