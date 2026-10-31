// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::time::Duration;

use super::deadline::{Deadline, arm};
use super::device::{block_on_for, guarded};
use super::resources::filled;

const WAIT: Duration = Duration::from_secs(60);

fn device_limited_to(max_buffer_size: u64) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = block_on_for(
        instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: true,
            ..wgpu::RequestAdapterOptions::default()
        }),
        WAIT,
    )
    .and_then(Result::ok)?;
    let limits = wgpu::Limits {
        max_buffer_size,
        ..wgpu::Limits::default()
    };
    block_on_for(
        adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: limits,
            ..wgpu::DeviceDescriptor::default()
        }),
        WAIT,
    )
    .and_then(Result::ok)
}

#[test]
fn a_future_that_never_resolves_gives_up_at_the_limit() {
    let started = std::time::Instant::now();

    let output = block_on_for(std::future::pending::<()>(), Duration::from_millis(50));

    assert!(output.is_none());
    assert!(started.elapsed() < Duration::from_secs(30));
}

#[test]
fn a_ready_future_returns_its_output() {
    assert_eq!(block_on_for(async { 7 }, Duration::from_secs(1)), Some(7));
}

#[test]
fn a_deadline_that_passes_runs_its_expiry() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let _deadline = Deadline::after(Duration::from_millis(20), move || {
        sender.send(()).unwrap_or_default();
    });

    assert!(receiver.recv_timeout(Duration::from_secs(30)).is_ok());
}

#[test]
fn a_deadline_dropped_in_time_never_runs_its_expiry() {
    let (sender, receiver) = std::sync::mpsc::channel();
    drop(Deadline::after(Duration::from_millis(100), move || {
        sender.send(()).unwrap_or_default();
    }));

    assert!(
        receiver.recv_timeout(Duration::from_millis(500)).is_err(),
        "the expiry stayed silent"
    );
}

#[expect(
    clippy::print_stderr,
    reason = "tells the test run why a test checked nothing"
)]
fn no_adapter(test: &str) {
    assert!(
        std::env::var_os("P4KX_REQUIRE_SOFTWARE_ADAPTER").is_none(),
        "{test}: an adapter is required"
    );
    eprintln!("{test}: no graphics adapter, nothing checked");
}

#[test]
fn a_buffer_the_device_refuses_fails_in_the_error_scope_instead_of_panicking() {
    arm("a_buffer_the_device_refuses");
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
    arm("a_buffer_within_the_limit");
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
