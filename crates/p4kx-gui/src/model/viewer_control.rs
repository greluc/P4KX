// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::Model;
use crate::{Browser, Camera, ModelPreview, Playback, ViewerSupport};

impl Model {
    /// Returns what the window learned about drawing the 3D viewer on its graphics device.
    #[must_use]
    pub const fn viewer_support(&self) -> &ViewerSupport {
        &self.viewer_support
    }

    /// Sets what the window learned about drawing the 3D viewer on its graphics device.
    pub fn set_viewer_support(&mut self, support: ViewerSupport) {
        self.viewer_support = support;
    }

    /// Returns the model preview of the selected Entry for moving its camera.
    pub fn model_preview_mut(&mut self) -> Option<&mut ModelPreview> {
        self.browser.as_mut().and_then(Browser::model_mut)
    }

    /// Returns the camera of the model shown in the 3D viewer.
    pub fn viewer_camera_mut(&mut self) -> Option<&mut Camera> {
        self.model_preview_mut().and_then(ModelPreview::camera_mut)
    }

    /// Returns the playback of the skeleton and animations of the model shown in the 3D viewer.
    pub fn viewer_playback_mut(&mut self) -> Option<&mut Playback> {
        self.model_preview_mut()
            .and_then(ModelPreview::playback_mut)
    }
}
