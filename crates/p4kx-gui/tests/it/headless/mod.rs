// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the headless viewer tests that read the state of the window.

use std::error::Error;
use std::sync::Arc;

use egui_kittest::Harness;
use p4kx_gui::{GpuReport, Language, Model, ModelStatus, ViewerSupport};
use tempfile::TempDir;

use crate::viewing::{model_preview, opened_with};

pub(crate) fn ready() -> ViewerSupport {
    ViewerSupport::Ready(Arc::new(GpuReport::default()))
}

pub(crate) fn opened(
    language: Language,
    support: ViewerSupport,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    opened_with(language, support, |builder| builder)
}

pub(crate) fn status_is_ready(harness: &Harness<'_, Model>) -> bool {
    model_preview(harness.state()).is_some_and(|preview| {
        matches!(preview.status(), ModelStatus::Ready(_)) && preview.camera().is_some()
    })
}
