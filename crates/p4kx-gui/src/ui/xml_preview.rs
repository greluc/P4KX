use egui::Ui;
use fluent_bundle::FluentValue;

use super::xml_view;
use crate::theme::desktop::SPINNER_INLINE;
use crate::theme::palette;
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::{Text, Texts, XML_SIZE_LIMIT, XML_TEXT_LIMIT, XmlPreview, XmlStatus, XmlText, format};

/// Shows the XML tab of the `CryXml` Entry at `path` of `size` bytes: the converted lines, or
/// where the conversion stands.
pub(super) fn show(ui: &mut Ui, texts: &Texts, (path, size): (&str, u64), xml: &XmlPreview) {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    frames::preview_body().show(ui, |ui| match xml.status() {
        XmlStatus::Converting => {
            ui.horizontal(|ui| {
                spinner(ui, SPINNER_INLINE);
                ui.label(texts.format(XmlText::Converting, &[path_arg()]));
            });
        }
        XmlStatus::TooLarge => {
            ui.label(texts.format(
                XmlText::TooLarge,
                &[
                    path_arg(),
                    ("size", format::size(size, language).into()),
                    ("limit", format::size(XML_SIZE_LIMIT, language).into()),
                ],
            ));
        }
        XmlStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.format(XmlText::Stopped, &[path_arg()]),
                |reason| {
                    let reason = ("reason", FluentValue::from(reason.to_owned()));
                    texts.format(XmlText::Failed, &[path_arg(), reason])
                },
            );
            ui.colored_label(palette::DANGER_TEXT, text);
        }
        XmlStatus::ArchiveChanged => {
            ui.colored_label(palette::WARNING, texts.get(Text::PreviewArchiveChanged));
        }
        XmlStatus::Ready(document) => {
            if document.is_truncated() {
                let lines = u64::try_from(document.line_count()).unwrap_or(u64::MAX);
                let limit = u64::try_from(XML_TEXT_LIMIT).unwrap_or(u64::MAX);
                let note = texts.format(
                    XmlText::Truncated,
                    &[
                        ("lines", format::number(lines, language).into()),
                        ("limit", format::size(limit, language).into()),
                    ],
                );
                ui.colored_label(palette::WARNING, note);
            }
            xml_view::show(ui, texts, document);
        }
    });
}
