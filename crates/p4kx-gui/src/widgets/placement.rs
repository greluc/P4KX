// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Where dialogs stand: between the header and the status bar, hanging from a fixed top edge.

use egui::{Align2, Context, Id, Modal, Rect, Ui, Window, pos2, vec2};

use super::frames::{FOOT_HEIGHT, MODAL_PADDING_Y, Tone, modal, modal_frame};
use crate::theme::desktop::{BUTTON_HEIGHT, HEADER_HEIGHT, STATUS_HEIGHT};
use crate::theme::tokens::SPACE_4;

/// Returns the part of the window between the header and the status bar, which dialogs keep
/// inside.
///
/// The rects of the header and the status bar are those [`crate::ui::regions`] recorded in the
/// frame being drawn, so this holds for dialogs shown after the panels of the window; before they
/// are drawn, or without a window, the theme's heights stand in.
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

/// Returns whether the window `title` was not drawn in the frame before, so that it opens anew,
/// and notes that it is drawn in this one.
fn opens_anew(ctx: &Context, title: &str) -> bool {
    let key = Id::new(("dialog-window-frame", title));
    let now = ctx.cumulative_frame_nr();
    let before = ctx.data_mut(|data| data.get_temp::<u64>(key));
    ctx.data_mut(|data| data.insert_temp(key, now));
    before.is_none_or(|before| now.saturating_sub(before) > 1)
}

/// Returns a non-modal `.modal` window titled `title` that opens at the top centre of
/// [`dialog_area`], a gap below the header, each time it is shown anew, and stays a gap inside it
/// wherever the user moves it; its head is drawn by [`super::frames::head`].
pub(crate) fn window(ctx: &Context, title: &str, tone: Tone) -> Window<'static> {
    let area = dialog_area(ctx);
    let keep = area.shrink2(vec2(0.0, SPACE_4));
    let at = pos2(area.center().x, keep.top());
    let window = Window::new(title.to_owned())
        .title_bar(false)
        .frame(modal_frame(tone))
        .constrain_to(keep)
        .pivot(Align2::CENTER_TOP);
    if opens_anew(ctx, title) {
        window.current_pos(at)
    } else {
        window.default_pos(at)
    }
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
    let padding = 2.0 * MODAL_PADDING_Y;
    let gaps = 2.0 * SPACE_4;
    let spacing = 2.0 * ui.spacing().item_spacing.y;
    (dialog_area(ui.ctx()).height() - above - FOOT_HEIGHT - padding - gaps - spacing)
        .max(BUTTON_HEIGHT)
}
