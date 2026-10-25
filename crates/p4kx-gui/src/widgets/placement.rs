// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Where dialogs stand: between the header and the status bar, hanging from a fixed top edge.

use egui::{Align2, Context, Id, Modal, Rect, Ui, Window, pos2, vec2};

use super::frames::{Tone, modal, modal_frame};
use crate::theme::desktop::{BUTTON_HEIGHT, HEADER_HEIGHT, STATUS_HEIGHT};
use crate::theme::tokens::{SPACE_3, SPACE_4};

/// Returns the part of the window between the header and the status bar as the last frame drew
/// them, which dialogs keep inside.
pub(crate) fn dialog_area(ctx: &Context) -> Rect {
    let regions = crate::ui::regions(ctx);
    let content = ctx.content_rect();
    let top = regions
        .header
        .map_or_else(|| content.top() + HEADER_HEIGHT, |header| header.bottom());
    let bottom = regions
        .status
        .map_or_else(|| content.bottom() - STATUS_HEIGHT, |status| status.top())
        .max(top);
    Rect::from_min_max(pos2(content.left(), top), pos2(content.right(), bottom))
}

/// Returns a non-modal `.modal` window titled `title` that opens at the top centre of
/// [`dialog_area`] and stays inside it; its head is drawn by [`super::frames::head`].
pub(crate) fn window(ctx: &Context, title: &str, tone: Tone) -> Window<'static> {
    let area = dialog_area(ctx);
    Window::new(title.to_owned())
        .title_bar(false)
        .frame(modal_frame(tone))
        .constrain_to(area)
        .pivot(Align2::CENTER_TOP)
        .default_pos(pos2(area.center().x, area.top() + SPACE_4))
}

/// Returns a `.modal` that hangs from a fixed edge a gap below the header, so that it keeps its
/// top edge when its height changes instead of centring again.
pub(crate) fn hanging_modal(ctx: &Context, id: Id, tone: Tone) -> Modal {
    let drop = dialog_area(ctx).top() - ctx.content_rect().top() + SPACE_4;
    modal(id, tone).area(Modal::default_area(id).anchor(Align2::CENTER_TOP, vec2(0.0, drop)))
}

/// Returns the height the body of a dialog can take below what `ui` holds so far, so that its
/// foot stays a gap above the status bar.
pub(crate) fn body_room(ui: &Ui) -> f32 {
    let above = ui.cursor().top() - ui.min_rect().top();
    let foot = SPACE_4 + SPACE_3 + BUTTON_HEIGHT;
    let frame = 2.0 * SPACE_3;
    let gaps = 2.0 * SPACE_4;
    let spacing = 2.0 * ui.spacing().item_spacing.y;
    (dialog_area(ui.ctx()).height() - above - foot - frame - gaps - spacing).max(BUTTON_HEIGHT)
}
