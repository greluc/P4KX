// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use eframe::egui_wgpu::{RenderState, RendererOptions, WgpuSetup, wgpu};
use egui_kittest::wgpu::{create_render_state, default_wgpu_setup};

use super::device::guarded;
use super::resources::filled;

fn state_limited_to(max_buffer_size: u64) -> Option<RenderState> {
    let WgpuSetup::CreateNew(mut setup) = default_wgpu_setup() else {
        return None;
    };
    let describe = Arc::clone(&setup.device_descriptor);
    setup.device_descriptor = Arc::new(move |adapter| {
        let mut descriptor = describe(adapter);
        descriptor.required_limits.max_buffer_size = max_buffer_size;
        descriptor
    });
    catch_unwind(AssertUnwindSafe(|| {
        create_render_state(WgpuSetup::CreateNew(setup), RendererOptions::PREDICTABLE)
    }))
    .ok()
}

#[expect(
    clippy::print_stderr,
    reason = "tells the test run why a test checked nothing"
)]
fn no_adapter(test: &str) {
    eprintln!("{test}: no graphics adapter, nothing checked");
}

#[test]
fn a_buffer_the_device_refuses_fails_in_the_error_scope_instead_of_panicking() {
    let Some(state) = state_limited_to(1024) else {
        return no_adapter("a_buffer_the_device_refuses");
    };

    let (_buffer, failed) = guarded(&state.device, || {
        filled(
            (&state.device, &state.queue),
            ("test", wgpu::BufferUsages::VERTEX),
            &[0; 4096],
        )
    });

    assert!(failed, "the error scope caught the refused buffer");
}

#[test]
fn a_buffer_within_the_limit_is_created_and_written_without_an_error() {
    let Some(state) = state_limited_to(1024) else {
        return no_adapter("a_buffer_within_the_limit");
    };

    let (buffer, failed) = guarded(&state.device, || {
        filled(
            (&state.device, &state.queue),
            ("test", wgpu::BufferUsages::INDEX),
            &[7; 512],
        )
    });

    assert!(!failed, "no error");
    assert_eq!(buffer.size(), 512, "the buffer holds the contents");
}
