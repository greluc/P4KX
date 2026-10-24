// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the 3D viewer of the Model tab, looked up in the Fluent file of the current
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelText {
    /// The preview tab that shows a model in the 3D viewer.
    Tab,
    /// The action that frames the whole model again.
    Reset,
    /// The overview of the mouse and keyboard controls.
    Hint,
    /// The key of the number of vertices.
    Vertices,
    /// The key of the number of triangles.
    Triangles,
    /// The key of the size of the model.
    Extent,
    /// The size of the model in metres; takes `width`, `depth` and `height`.
    Dimensions,
    /// The accessible name of the 3D view; takes `path`, `vertices` and `triangles`.
    View,
    /// Shown while the model is read; takes `path`.
    Loading,
    /// Shown when the model cannot be read; takes `path` and `reason`.
    Failed,
    /// Shown when the model file or its companion is above the limit; takes `path`, `size` and
    /// `limit`.
    FileTooLarge,
    /// Shown for a model without triangles; takes `path`.
    Empty,
    /// Shown for a model with more than the viewer takes; takes `path`, `vertices`,
    /// `triangles`, `max_vertices` and `max_triangles`.
    TooLarge,
    /// Shown for a model without usable coordinates; takes `path`.
    NotFinite,
    /// Shown when the window has no graphics device for the viewer.
    GpuNone,
    /// Shown when the device takes too small textures; takes `limit`.
    GpuTextureSide,
    /// Shown when the device takes too small buffers; takes `limit`.
    GpuBufferSize,
    /// Shown when the device refused the viewer's shaders; takes `reason`.
    GpuSetup,
    /// Shown when the graphics device was lost.
    GpuLost,
    /// Shown when the graphics device failed to draw.
    GpuFailed,
    /// Shown when the model is too large for the graphics device; takes `path`.
    GpuRejected,
}

impl ModelText {
    /// Every text of the 3D viewer.
    pub(crate) const ALL: [Self; 21] = [
        Self::Tab,
        Self::Reset,
        Self::Hint,
        Self::Vertices,
        Self::Triangles,
        Self::Extent,
        Self::Dimensions,
        Self::View,
        Self::Loading,
        Self::Failed,
        Self::FileTooLarge,
        Self::Empty,
        Self::TooLarge,
        Self::NotFinite,
        Self::GpuNone,
        Self::GpuTextureSide,
        Self::GpuBufferSize,
        Self::GpuSetup,
        Self::GpuLost,
        Self::GpuFailed,
        Self::GpuRejected,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "preview-tab-model",
            Self::Reset => "model-reset",
            Self::Hint => "model-hint",
            Self::Vertices => "model-vertices",
            Self::Triangles => "model-triangles",
            Self::Extent => "model-extent",
            Self::Dimensions => "model-dimensions",
            Self::View => "model-view",
            Self::Loading => "model-loading",
            Self::Failed => "model-failed",
            Self::FileTooLarge => "model-file-too-large",
            Self::Empty => "model-empty",
            Self::TooLarge => "model-too-large",
            Self::NotFinite => "model-not-finite",
            Self::GpuNone => "model-gpu-none",
            Self::GpuTextureSide => "model-gpu-texture-side",
            Self::GpuBufferSize => "model-gpu-buffer-size",
            Self::GpuSetup => "model-gpu-setup",
            Self::GpuLost => "model-gpu-lost",
            Self::GpuFailed => "model-gpu-failed",
            Self::GpuRejected => "model-gpu-rejected",
        }
    }
}
