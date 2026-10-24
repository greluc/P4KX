// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::device::{block_on, guarded};
use super::resources::filled;

fn device_limited_to(max_buffer_size: u64) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let limits = wgpu::Limits {
        max_buffer_size,
        ..wgpu::Limits::default()
    };
    block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: limits,
        ..wgpu::DeviceDescriptor::default()
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
    let Some((device, queue)) = device_limited_to(1024) else {
        return no_adapter("a_buffer_the_device_refuses");
    };

    let (_buffer, failed) = guarded(&device, || {
        filled(
            (&device, &queue),
            ("test", wgpu::BufferUsages::VERTEX),
            &[0; 4096],
        )
    });

    assert!(failed, "the error scope caught the refused buffer");
}

#[test]
fn a_buffer_within_the_limit_is_created_and_written_without_an_error() {
    let Some((device, queue)) = device_limited_to(1024) else {
        return no_adapter("a_buffer_within_the_limit");
    };

    let (buffer, failed) = guarded(&device, || {
        filled(
            (&device, &queue),
            ("test", wgpu::BufferUsages::INDEX),
            &[7; 512],
        )
    });

    assert!(!failed, "no error");
    assert_eq!(buffer.size(), 512, "the buffer holds the contents");
}
