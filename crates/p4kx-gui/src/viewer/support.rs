use std::sync::Arc;

use super::gpu::{self, GpuReport, Unavailable};

/// Whether the window can draw the viewer, and what it learned about the graphics device.
#[derive(Debug, Clone, Default)]
pub enum ViewerSupport {
    /// The graphics device was not probed yet.
    #[default]
    Pending,
    /// The viewer is set up on the device.
    Ready(Arc<GpuReport>),
    /// The device cannot draw the viewer; the rest of the window works.
    Unavailable(Unavailable),
}

/// What keeps the viewer from drawing, if anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The device cannot draw the viewer, for the reason given.
    Unavailable(Unavailable),
    /// The device was lost while the window ran.
    Lost,
    /// A drawing command failed, e.g. for lack of memory.
    Failed,
    /// The model is too large for the device's buffers.
    Rejected,
}

impl ViewerSupport {
    /// Probes the graphics device of `frame` and sets the viewer up on it.
    #[must_use]
    pub fn probe(frame: &eframe::Frame) -> Self {
        frame
            .wgpu_render_state()
            .map_or(Self::Unavailable(Unavailable::NoDevice), |state| {
                gpu::install(state).map_or_else(Self::Unavailable, Self::Ready)
            })
    }

    /// Returns whether the device was probed.
    #[must_use]
    pub const fn is_probed(&self) -> bool {
        !matches!(self, Self::Pending)
    }

    /// Returns what keeps the viewer from drawing the scene with this `serial`, or `None` when
    /// it can.
    #[must_use]
    pub fn problem(&self, serial: u64) -> Option<Problem> {
        match self {
            Self::Pending => Some(Problem::Unavailable(Unavailable::NoDevice)),
            Self::Unavailable(reason) => Some(Problem::Unavailable(reason.clone())),
            Self::Ready(report) if report.is_lost() => Some(Problem::Lost),
            Self::Ready(report) if report.has_failed() => Some(Problem::Failed),
            Self::Ready(report) if report.is_rejected(serial) => Some(Problem::Rejected),
            Self::Ready(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Problem, ViewerSupport};
    use crate::viewer::gpu::{GpuReport, Unavailable};

    #[test]
    fn a_ready_device_has_no_problem() {
        let support = ViewerSupport::Ready(Arc::new(GpuReport::default()));

        assert_eq!(support.problem(7), None, "nothing in the way");
    }

    #[test]
    fn an_unavailable_device_names_its_reason() {
        let support = ViewerSupport::Unavailable(Unavailable::BufferSize(1));

        assert_eq!(
            support.problem(7),
            Some(Problem::Unavailable(Unavailable::BufferSize(1))),
            "reason"
        );
    }

    #[test]
    fn an_unprobed_device_cannot_draw() {
        assert!(ViewerSupport::Pending.problem(7).is_some(), "not probed");
        assert!(!ViewerSupport::Pending.is_probed(), "pending");
    }
}
