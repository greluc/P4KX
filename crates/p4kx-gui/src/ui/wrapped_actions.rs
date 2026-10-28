// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Whether a toolbar wrapped its actions into a second row, so that the room of the work surface
//! reserves the height of that row only then.

use egui::{Context, Id};

use crate::theme::window_metrics::WRAPPED_ACTIONS_HEIGHT;

/// The last record: the frame it was made in and whether the actions wrapped.
#[derive(Clone, Copy)]
struct Record {
    frame: u64,
    wrapped: bool,
}

/// Records that a toolbar drawn in this frame wrapped its actions or did not, and asks for another
/// frame when that differs from the last record.
pub(super) fn record(ctx: &Context, wrapped: bool) {
    let before = last(ctx);
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| data.insert_temp(Id::NULL, Record { frame, wrapped }));
    if before != wrapped {
        ctx.request_repaint();
    }
}

/// Returns the height the room of a work tab reserves for a second row of actions: the height of
/// that row while a toolbar of this or the last frame wrapped its actions, otherwise nothing.
pub(super) fn extra_height(ctx: &Context) -> f32 {
    if last(ctx) {
        WRAPPED_ACTIONS_HEIGHT
    } else {
        0.0
    }
}

fn last(ctx: &Context) -> bool {
    let frame = ctx.cumulative_frame_nr();
    ctx.data(|data| data.get_temp::<Record>(Id::NULL))
        .is_some_and(|record| record.wrapped && record.frame.saturating_add(1) >= frame)
}
