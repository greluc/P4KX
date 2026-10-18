use egui::{Align, Layout, ScrollArea, Ui};

use crate::theme::tokens::{FS_SM, SPACE_4, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone};
use crate::widgets::text::plain;
use crate::{ExtractionText, Model, Text, theme};

const LICENCE_HEIGHT: f32 = 420.0;

pub(super) fn show(ui: &Ui, model: &mut Model) {
    if !model.licence_shown() {
        return;
    }
    let title = model.texts().get(Text::FontLicence);
    let close_label = model.texts().get(ExtractionText::Close);
    let close_dialog = model.texts().get(Text::CloseDialog);
    let shown =
        frames::modal(frames::modal_id("font-licence"), Tone::Normal).show(ui.ctx(), |ui| {
            ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
            let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
            ScrollArea::vertical()
                .max_height(LICENCE_HEIGHT)
                .show(ui, |ui| {
                    ui.label(plain(theme::FONT_LICENCE, FS_SM, Weight::Regular));
                });
            ui.add_space(SPACE_4);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                button::button(ui, Kind::Cta, Size::Normal, &close_label, true).clicked()
            })
            .inner
                || closed
        });
    frames::modal_decoration(
        ui.ctx(),
        shown.response.layer_id,
        shown.response.rect,
        Tone::Normal,
    );
    if shown.inner || shown.should_close() {
        model.set_licence_shown(false);
    }
}
