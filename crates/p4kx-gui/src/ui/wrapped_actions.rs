// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Whether a toolbar wrapped its actions into a second row, so that the room of the work surface
//! reserves the height of that row only then.

use egui::{Context, Id};

use crate::theme::window_metrics::WRAPPED_ACTIONS_HEIGHT;

/// The last record: the frame it was made in and whether an action row wrapped in it.
#[derive(Clone, Copy, Default)]
struct Record {
    frame: u64,
    wrapped: bool,
}

fn key() -> Id {
    Id::NULL.with("wrapped-actions")
}

fn read(ctx: &Context) -> Record {
    ctx.data(|data| data.get_temp::<Record>(key()))
        .unwrap_or_default()
}

fn write(ctx: &Context, record: Record) {
    let before = read(ctx);
    ctx.data_mut(|data| data.insert_temp(key(), record));
    if before.wrapped != record.wrapped {
        ctx.request_repaint();
    }
}

/// Records that a toolbar drawn in this frame wrapped its actions or did not. Within a frame a
/// wrap wins over a toolbar that did not, and another frame is asked for when the state changes.
pub(super) fn record(ctx: &Context, wrapped: bool) {
    let frame = ctx.cumulative_frame_nr();
    let before = read(ctx);
    let seen = before.frame == frame && before.wrapped;
    write(
        ctx,
        Record {
            frame,
            wrapped: wrapped || seen,
        },
    );
}

/// Ends a frame: a wrap that no toolbar recorded in it, because the window widened or another tab
/// is shown, no longer counts.
pub(super) fn settle(ctx: &Context) {
    let frame = ctx.cumulative_frame_nr();
    let before = read(ctx);
    if before.wrapped && before.frame != frame {
        write(
            ctx,
            Record {
                frame,
                wrapped: false,
            },
        );
    }
}

/// Returns the height the room of a work tab reserves for a second row of actions: the height of
/// that row while a toolbar of the last frame wrapped its actions, otherwise nothing.
pub(super) fn extra_height(ctx: &Context) -> f32 {
    if read(ctx).wrapped {
        WRAPPED_ACTIONS_HEIGHT
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use egui::{Context, RawInput};

    use super::{extra_height, record, settle};
    use crate::theme::window_metrics::WRAPPED_ACTIONS_HEIGHT;

    fn frame(ctx: &Context, body: impl Fn(&Context)) {
        let mut output = ctx.run_ui(RawInput::default(), |ui| body(ui.ctx()));
        output.textures_delta.clear();
    }

    #[test]
    fn a_wrap_reserves_the_row_and_the_reserve_drops_when_no_toolbar_records_it_again() {
        let ctx = Context::default();

        frame(&ctx, |ctx| {
            record(ctx, true);
            settle(ctx);
        });
        assert_eq!(extra_height(&ctx), WRAPPED_ACTIONS_HEIGHT, "reserved");

        frame(&ctx, settle);
        assert_eq!(extra_height(&ctx), 0.0, "no record in the frame drops it");
    }

    #[test]
    fn a_toolbar_that_does_not_wrap_never_hides_one_that_does_in_the_same_frame() {
        let ctx = Context::default();

        frame(&ctx, |ctx| {
            record(ctx, true);
            record(ctx, false);
        });

        assert_eq!(extra_height(&ctx), WRAPPED_ACTIONS_HEIGHT);
    }

    #[test]
    fn a_toolbar_that_stops_wrapping_clears_the_reserve() {
        let ctx = Context::default();
        frame(&ctx, |ctx| record(ctx, true));

        frame(&ctx, |ctx| record(ctx, false));

        assert_eq!(extra_height(&ctx), 0.0);
    }
}
