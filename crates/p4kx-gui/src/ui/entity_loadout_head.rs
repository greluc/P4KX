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
    let spinner = if session.is_resolving() {
        SPINNER_INLINE + gap
    } else {
        0.0
    };
    let needed = 2.0f32.mul_add(gap, spinner)
        + line(ui, name.clone(), f32::INFINITY).size().x
        + line(ui, note.clone(), f32::INFINITY).size().x
        + button::width(ui, Size::Compact, &reset);
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
    if needed > ui.available_width() {
        ui.horizontal(titled);
        ui.horizontal(controls);
    } else {
        ui.horizontal(|ui| {
            titled(ui);
            controls(ui);
        });
    }
    ui.add_space(SPACE_2);
    clicked
}
