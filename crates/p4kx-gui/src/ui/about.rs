use egui::{Align, Label, Layout, RichText, Ui};

use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_3, Weight, font};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone};
use crate::{AboutText, ExtractionText, Model, Text};

const PARAGRAPHS: [AboutText; 6] = [
    AboutText::Licence,
    AboutText::FanProject,
    AboutText::NoGameData,
    AboutText::Responsibility,
    AboutText::Owners,
    AboutText::Trademark,
];

pub(super) fn show(ui: &Ui, model: &mut Model) {
    if !model.about_shown() {
        return;
    }
    let texts = model.texts();
    let title = texts.get(AboutText::Title);
    let version = texts.format(
        AboutText::Version,
        &[("version", env!("CARGO_PKG_VERSION").into())],
    );
    let paragraphs = PARAGRAPHS.map(|text| texts.get(text));
    let close_label = texts.get(ExtractionText::Close);
    let close_dialog = texts.get(Text::CloseDialog);
    let shown = frames::modal(frames::modal_id("about"), Tone::Normal).show(ui.ctx(), |ui| {
        ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
        let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
        paragraph(ui, &version, palette::GRAY_1, Weight::Bold);
        for text in &paragraphs {
            paragraph(ui, text, palette::GRAY_1, Weight::Regular);
        }
        frames::foot_rule(ui);
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
        model.set_about_shown(false);
    }
}

fn paragraph(ui: &mut Ui, text: &str, color: egui::Color32, weight: Weight) {
    ui.add(Label::new(
        RichText::new(text).font(font(FS_SM, weight)).color(color),
    ));
    ui.add_space(SPACE_3);
}
