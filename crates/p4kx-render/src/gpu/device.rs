// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::Wake;
use std::task::{Context, Poll, Waker};
use std::thread::{self, Thread};

use super::pipelines::{COLOUR_FORMAT, DEPTH_FORMAT};

const WANTED_SAMPLES: u32 = 4;

#[expect(
    clippy::cast_precision_loss,
    reason = "a target side is at most the device limit, far below the 2^24 an f32 holds exactly"
)]
pub(super) const fn to_f32(side: u32) -> f32 {
    side as f32
}

pub(super) fn supported_samples(adapter: &wgpu::Adapter) -> u32 {
    let supports = |format| {
        adapter
            .get_texture_format_features(format)
            .flags
            .sample_count_supported(WANTED_SAMPLES)
    };
    if supports(COLOUR_FORMAT) && supports(DEPTH_FORMAT) {
        WANTED_SAMPLES
    } else {
        1
    }
}

/// Runs `work` inside validation and out-of-memory error scopes and returns its output with
/// whether either scope caught an error.
pub(super) fn guarded<T>(device: &wgpu::Device, work: impl FnOnce() -> T) -> (T, bool) {
    let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let output = work();
    let invalid = settle(validation.pop()).flatten();
    let exhausted = settle(memory.pop()).flatten();
    (output, invalid.or(exhausted).is_some())
}

struct Unpark(Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Runs `future` to completion on the calling thread.
pub(super) fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut context = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        thread::park();
    }
}

/// Polls `future` once without waking; the native error scopes resolve at once.
pub(super) fn settle<F: Future>(future: F) -> Option<F::Output> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
}
