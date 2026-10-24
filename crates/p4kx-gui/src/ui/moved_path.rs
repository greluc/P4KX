// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::{Color32, Galley, Pos2, Ui, pos2};

use crate::theme::desktop::CHEVRON;
use crate::theme::palette;
use crate::theme::tokens::{SPACE_2, Weight};
use crate::widgets::rows::chevron;
use crate::widgets::text::{line, plain};

/// The record paths of a moved Record laid out to share one width, with the orange direction
/// chevron of the facts bar between them.
#[derive(Clone)]
pub(super) struct MovedPath {
    old: Arc<Galley>,
    new: Arc<Galley>,
}

impl MovedPath {
    /// Lays `old` and `new` out at `size` in `weight` within `width`; a path that does not fit
    /// ends with an ellipsis, the shorter one keeping its length when it can.
    pub(super) fn lay_out(
        ui: &Ui,
        (old, new): (&str, &str),
        (size, weight): (f32, Weight),
        width: f32,
    ) -> Self {
        let room = 2.0f32.mul_add(-SPACE_2, width - CHEVRON).max(0.0);
        let natural = |text: &str| line(ui, plain(text, size, weight), f32::INFINITY);
        let (old_galley, new_galley) = (natural(old), natural(new));
        let (old_width, new_width) = (old_galley.size().x, new_galley.size().x);
        if old_width + new_width <= room {
            return Self {
                old: old_galley,
                new: new_galley,
            };
        }
        let half = room / 2.0;
        let old_room = if new_width < half {
            room - new_width
        } else {
            half.min(old_width)
        };
        let new_room = room - old_room;
        Self {
            old: line(ui, plain(old, size, weight), old_room),
            new: line(ui, plain(new, size, weight), new_room),
        }
    }

    /// Returns the width it takes.
    pub(super) fn width(&self) -> f32 {
        2.0f32.mul_add(SPACE_2, self.old.size().x + CHEVRON) + self.new.size().x
    }

    /// Paints both paths in `colour` and the chevron in `color-primary`, the left edge at
    /// `left_centre`.
    pub(super) fn paint(&self, ui: &Ui, left_centre: Pos2, colour: Color32) {
        let top = |galley: &Galley| left_centre.y - galley.size().y / 2.0;
        let painter = ui.painter();
        painter.galley(
            pos2(left_centre.x, top(&self.old)),
            Arc::clone(&self.old),
            colour,
        );
        let mut left = left_centre.x + self.old.size().x + SPACE_2;
        chevron(
            painter,
            pos2(left + CHEVRON / 2.0, left_centre.y),
            CHEVRON,
            false,
            palette::PRIMARY,
        );
        left += CHEVRON + SPACE_2;
        painter.galley(pos2(left, top(&self.new)), Arc::clone(&self.new), colour);
    }
}
