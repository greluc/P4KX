// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Layout, Ui};

use crate::theme::desktop::SPINNER_INLINE;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_2, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::indicator;
use crate::widgets::text::{line, plain};
use crate::{EntityText, LoadoutSession, Texts};

/// Shows the name of the Loadout of `session` and its changes with the button that resets them;
/// the changes go on a second line when they do not fit beside the name. Returns whether the
/// button was clicked.
pub(super) fn show(ui: &mut Ui, texts: &Texts, session: &LoadoutSession) -> bool {
    let (excluded, replaced) = session.change_counts();
    let any = excluded + replaced > 0;
    let changes = if any {
        texts.format(
            EntityText::Changes,
            &[
                ("excluded", excluded.to_string().into()),
                ("replaced", replaced.to_string().into()),
            ],
        )
    } else {
        texts.get(EntityText::NoChanges)
    };
    let reset = texts.get(EntityText::Reset);
    let name = plain(session.name(), FS_SM, Weight::Bold).color(palette::WHITE);
    let note = plain(changes, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT);
    let gap = ui.spacing().item_spacing.x;
    let one_line = fits_on_one_line(
        gap,
        session.is_resolving(),
        [
            line(ui, name.clone(), f32::INFINITY).size().x,
            line(ui, note.clone(), f32::INFINITY).size().x,
            button::width(ui, Size::Compact, &reset),
        ],
        ui.available_width(),
    );
    let mut clicked = false;
    let titled = |ui: &mut Ui| {
        ui.label(name);
        if session.is_resolving() {
            indicator::spinner(ui, SPINNER_INLINE);
        }
    };
    let controls = |ui: &mut Ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            clicked = button::button(ui, Kind::Ghost, Size::Compact, &reset, any).clicked();
            ui.label(note);
        });
    };
    if one_line {
        ui.horizontal(|ui| {
            titled(ui);
            controls(ui);
        });
    } else {
        ui.horizontal(titled);
        ui.horizontal(controls);
    }
    ui.add_space(SPACE_2);
    clicked
}

/// Returns whether the head fits on one line of `available` width; see [`needed_width`].
fn fits_on_one_line(gap: f32, resolving: bool, widths: [f32; 3], available: f32) -> bool {
    needed_width(gap, resolving, widths) <= available
}

/// Returns the width of the head on one line: two gaps, the spinner with its gap while
/// `resolving`, and the widths of the name, the note and the button.
fn needed_width(gap: f32, resolving: bool, widths: [f32; 3]) -> f32 {
    let spinner = if resolving { SPINNER_INLINE + gap } else { 0.0 };
    2.0f32.mul_add(gap, spinner) + widths.iter().sum::<f32>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTHS: [f32; 3] = [100.0, 50.0, 30.0];

    #[test]
    fn two_gaps_and_the_three_widths_make_the_line() {
        assert!((needed_width(8.0, false, WIDTHS) - 196.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_spinner_adds_itself_and_its_gap() {
        let without = needed_width(8.0, false, WIDTHS);
        let with = needed_width(8.0, true, WIDTHS);
        assert!((with - without - (SPINNER_INLINE + 8.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn a_line_that_just_fits_stays_on_one_line() {
        assert!(fits_on_one_line(8.0, false, WIDTHS, 196.0));
        assert!(!fits_on_one_line(8.0, false, WIDTHS, 195.0));
    }

    #[test]
    fn the_spinner_can_push_a_fitting_line_onto_two() {
        let available = needed_width(8.0, false, WIDTHS);
        assert!(fits_on_one_line(8.0, false, WIDTHS, available));
        assert!(!fits_on_one_line(8.0, true, WIDTHS, available));
        assert!(fits_on_one_line(
            8.0,
            true,
            WIDTHS,
            available + SPINNER_INLINE + 8.0
        ));
    }
}
