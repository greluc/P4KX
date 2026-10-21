//! The Info tab and Export as GLB of the model preview over synthetic Archives, driven headless.
//!
//! One test binary for these suites: each integration test of the window links all of it, and
//! the CI runners have little disk to spare for more binaries.

#[path = "../glb_exporting/mod.rs"]
mod glb_exporting;
#[path = "../headless/mod.rs"]
mod headless;
#[path = "../info_models/mod.rs"]
mod info_models;
#[path = "../viewing/mod.rs"]
mod viewing;

mod export;
mod export_window;
mod info;
mod info_render;
