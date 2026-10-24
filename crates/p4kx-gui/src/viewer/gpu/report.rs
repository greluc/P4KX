// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use thiserror::Error;

/// Why the viewer cannot use the graphics device.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unavailable {
    /// The window has no wgpu device, e.g. it was started with another renderer.
    #[error("the window has no wgpu device")]
    NoDevice,
    /// The device does not take textures of the size the viewer needs.
    #[error("the device takes textures of at most {0} texels a side")]
    TextureSide(u32),
    /// The device does not take buffers of the size the viewer needs.
    #[error("the device takes buffers of at most {0} bytes")]
    BufferSize(u64),
    /// The shaders or pipelines were refused.
    #[error("{0}")]
    Setup(String),
}

/// What the render side tells the window: whether the device is lost and which scenes it failed to
/// draw or refused. Shared between the paint callback and the window.
#[derive(Debug, Default)]
pub struct GpuReport {
    lost: AtomicBool,
    failed: AtomicU64,
    rejected: AtomicU64,
    resident: AtomicU64,
    uploads: AtomicU64,
    passes: AtomicU64,
    buffers: AtomicUsize,
    instances: AtomicU64,
    draw_calls: AtomicU64,
    textures: AtomicUsize,
}

impl GpuReport {
    /// Returns whether the graphics device was lost.
    #[must_use]
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    /// Returns whether drawing the scene with this serial failed validation or ran out of memory;
    /// the next scene is drawn again.
    #[must_use]
    pub fn has_failed(&self, serial: u64) -> bool {
        serial != 0 && self.failed.load(Ordering::Acquire) == serial
    }

    /// Returns whether the scene with this serial is too large for the device.
    #[must_use]
    pub fn is_rejected(&self, serial: u64) -> bool {
        serial != 0 && self.rejected.load(Ordering::Acquire) == serial
    }

    /// Returns the serial of the scene whose buffers are on the device, or `None`.
    #[must_use]
    pub fn resident(&self) -> Option<u64> {
        Some(self.resident.load(Ordering::Acquire)).filter(|&serial| serial != 0)
    }

    /// Returns how many times a scene was uploaded.
    #[must_use]
    pub fn uploads(&self) -> u64 {
        self.uploads.load(Ordering::Acquire)
    }

    /// Returns how many times the scene was drawn into the picture; a repaint with the same view
    /// does not draw again.
    #[must_use]
    pub fn passes(&self) -> u64 {
        self.passes.load(Ordering::Acquire)
    }

    /// Returns how many buffers the viewer holds on the device.
    #[must_use]
    pub fn buffers(&self) -> usize {
        self.buffers.load(Ordering::Acquire)
    }

    /// Returns how many instances the last pass of an Assembly drew.
    #[must_use]
    pub fn instances(&self) -> u64 {
        self.instances.load(Ordering::Acquire)
    }

    /// Returns how many draw calls the last pass of an Assembly made.
    #[must_use]
    pub fn draw_calls(&self) -> u64 {
        self.draw_calls.load(Ordering::Acquire)
    }

    /// Returns how many textures the viewer holds on the device for an Assembly.
    #[must_use]
    pub fn textures(&self) -> usize {
        self.textures.load(Ordering::Acquire)
    }

    pub(super) fn drew_instances(&self, instances: u64, calls: u64) {
        self.instances.store(instances, Ordering::Release);
        self.draw_calls.store(calls, Ordering::Release);
    }

    pub(super) fn texture_created(&self, count: usize) {
        self.textures.fetch_add(count, Ordering::AcqRel);
    }

    pub(super) fn texture_destroyed(&self, count: usize) {
        self.textures.fetch_sub(count, Ordering::AcqRel);
    }

    /// Records that the graphics device was lost.
    pub fn device_lost(&self) {
        self.lost.store(true, Ordering::Release);
    }

    /// Records that drawing the scene with this serial failed.
    pub fn fail(&self, serial: u64) {
        self.failed.store(serial, Ordering::Release);
        self.resident.store(0, Ordering::Release);
    }

    /// Records that the scene with this serial is too large for the device.
    pub fn reject(&self, serial: u64) {
        self.rejected.store(serial, Ordering::Release);
        self.resident.store(0, Ordering::Release);
    }

    pub(super) fn uploaded(&self, serial: u64) {
        self.resident.store(serial, Ordering::Release);
        self.uploads.fetch_add(1, Ordering::AcqRel);
    }

    pub(super) fn drew(&self) {
        self.passes.fetch_add(1, Ordering::AcqRel);
    }

    pub(super) fn buffer_created(&self, count: usize) {
        self.buffers.fetch_add(count, Ordering::AcqRel);
    }

    pub(super) fn buffer_destroyed(&self, count: usize) {
        self.buffers.fetch_sub(count, Ordering::AcqRel);
    }
}
