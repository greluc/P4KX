// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::{Align, Hyperlink, Id, Label, Layout, RichText, Ui};

use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_3, Weight, font};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone};
use crate::{AboutText, ExtractionText, Language, Model, Text};

const PROJECT_URL: &str = "https://github.com/p4kx-org/p4kx";

const SOURCE_AFTER: usize = 1;

const PARAGRAPHS: [AboutText; 7] = [
    AboutText::Copyright,
    AboutText::Licence,
    AboutText::FanProject,
    AboutText::NoGameData,
    AboutText::Responsibility,
    AboutText::Owners,
    AboutText::Trademark,
];

/// The texts of the About window in one language, looked up once and not every frame.
struct Strings {
    title: String,
    version: String,
    source: String,
    paragraphs: [String; PARAGRAPHS.len()],
    close_label: String,
    close_dialog: String,
}

impl Strings {
    fn new(model: &Model) -> Self {
        let texts = model.texts();
        Self {
            title: texts.get(AboutText::Title),
            version: texts.format(
                AboutText::Version,
                &[("version", env!("CARGO_PKG_VERSION").into())],
            ),
            source: texts.get(AboutText::Source),
            paragraphs: PARAGRAPHS.map(|text| texts.get(text)),
            close_label: texts.get(ExtractionText::Close),
            close_dialog: texts.get(Text::CloseDialog),
        }
    }
}

/// Returns the texts for the language of `model`, kept until the language changes.
fn strings(ui: &Ui, model: &Model) -> Arc<Strings> {
    let id = Id::unique("p4kx-about-strings");
    let language = model.texts().language();
    if let Some((cached, strings)) = ui.data(|data| data.get_temp::<(Language, Arc<Strings>)>(id))
        && cached == language
    {
        return strings;
    }
    let strings = Arc::new(Strings::new(model));
    ui.data_mut(|data| data.insert_temp(id, (language, Arc::clone(&strings))));
    strings
}

pub(super) fn show(ui: &Ui, model: &mut Model) {
    if !model.about_shown() {
        return;
    }
    let strings = strings(ui, model);
    let shown = frames::modal(frames::modal_id("about"), Tone::Normal).show(ui.ctx(), |ui| {
        ui.set_width(frames::MODAL_BODY_WIDTH);
        let closed = frames::head(
            ui,
            (&strings.title, Tone::Normal),
            true,
            Some(&strings.close_dialog),
        );
        frames::scrolling_body(ui, "about-body", |ui| {
            ui.spacing_mut().item_spacing.y = SPACE_3;
            paragraph(ui, &strings.version, Weight::Bold);
            for (index, text) in strings.paragraphs.iter().enumerate() {
                paragraph(ui, text, Weight::Regular);
                if index == SOURCE_AFTER {
                    source_link(ui, &strings.source);
                }
            }
        });
        frames::foot_rule(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            button::button(ui, Kind::Cta, Size::Normal, &strings.close_label, true).clicked()
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

fn source_link(ui: &mut Ui, label: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = SPACE_3;
        paragraph(ui, label, Weight::Regular);
        ui.add(Hyperlink::from_label_and_url(
            RichText::new(PROJECT_URL).font(font(FS_SM, Weight::Regular)),
            PROJECT_URL,
        ));
    });
}

fn paragraph(ui: &mut Ui, text: &str, weight: Weight) {
    ui.add(Label::new(
        RichText::new(text)
            .font(font(FS_SM, weight))
            .color(palette::GRAY_1),
    ));
}
