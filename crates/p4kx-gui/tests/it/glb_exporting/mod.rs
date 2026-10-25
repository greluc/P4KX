// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the Export as GLB tests: start an export into a file and wait for it to end.

use crate::waiting::{RunQuiet, wait_for};
use std::error::Error;
use std::path::Path;

use egui_kittest::Harness;
use p4kx_gui::{Model, ModelExport};

pub(crate) type Window = (tempfile::TempDir, Harness<'static, Model>);

/// Opens the setup of the selected model, chooses `file` and submits it; returns whether the
/// export started.
pub(crate) fn start(harness: &mut Harness<'_, Model>, file: &Path) -> Result<bool, &'static str> {
    harness.state_mut().begin_model_export();
    let setup = harness
        .state_mut()
        .model_export_setup_mut()
        .ok_or("the setup did not open")?;
    setup.set_file(file.to_path_buf());
    Ok(harness.state_mut().submit_model_export(|| {}))
}

/// Steps the window until the export ended.
pub(crate) fn finished(harness: &mut Harness<'_, Model>) -> Result<(), Box<dyn Error>> {
    wait_for(harness, "the export to end", |harness| {
        !harness
            .state()
            .model_export()
            .is_some_and(ModelExport::is_running)
    })?;
    harness.run_quiet();
    Ok(())
}
