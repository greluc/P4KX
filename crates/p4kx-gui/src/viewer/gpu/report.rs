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

/// What the render side tells the window: whether the device is lost or failed and which scene
/// was refused. Shared between the paint callback and the window.
#[derive(Debug, Default)]
pub struct GpuReport {
    lost: AtomicBool,
    failed: AtomicBool,
    rejected: AtomicU64,
    resident: AtomicU64,
    uploads: AtomicU64,
    buffers: AtomicUsize,
}

impl GpuReport {
    /// Returns whether the graphics device was lost.
    #[must_use]
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    /// Returns whether a drawing command failed validation or ran out of memory.
    #[must_use]
    pub fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    /// Returns whether the viewer cannot draw any more: the device is lost or a command failed.
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.is_lost() || self.has_failed()
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

    /// Returns how many buffers the viewer holds on the device.
    #[must_use]
    pub fn buffers(&self) -> usize {
        self.buffers.load(Ordering::Acquire)
    }

    /// Records that the graphics device was lost.
    pub fn device_lost(&self) {
        self.lost.store(true, Ordering::Release);
    }

    /// Records that a drawing command failed.
    pub fn fail(&self) {
        self.failed.store(true, Ordering::Release);
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

    pub(super) fn buffer_created(&self, count: usize) {
        self.buffers.fetch_add(count, Ordering::AcqRel);
    }

    pub(super) fn buffer_destroyed(&self, count: usize) {
        self.buffers.fetch_sub(count, Ordering::AcqRel);
    }
}
