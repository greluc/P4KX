//! Helpers of the Export as GLB tests: start an export into a file and wait for it to end.

use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

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
    let deadline = Instant::now() + Duration::from_secs(20);
    while harness
        .state()
        .model_export()
        .is_some_and(ModelExport::is_running)
    {
        if Instant::now() > deadline {
            return Err("the export did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}
