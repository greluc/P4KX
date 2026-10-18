use egui::{
    Align2, Button, FontId, Label, Rect, Role, ScrollArea, Sense, TextStyle, Ui, WidgetInfo, pos2,
    vec2,
};
use fluent_bundle::FluentValue;
use p4kx_extraction::Entry;

use super::row_height;
use crate::preview::{HEX_ROW_LEN, printable};
use crate::{
    Browser, Model, PREVIEW_SIZE_LIMIT, PreviewContents, PreviewStatus, PreviewTab, Text, Texts,
    format,
};

const TABS: [(PreviewTab, Text); 2] = [
    (PreviewTab::Text, Text::PreviewTabText),
    (PreviewTab::Hex, Text::PreviewTabHex),
];

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let mut tab = model.preview_tab();
    let contents = browser.preview().and_then(|preview| preview.contents());
    let mut copy = None;
    ui.horizontal(|ui| {
        for (candidate, text) in TABS {
            if ui
                .selectable_label(tab == candidate, texts.get(text))
                .clicked()
            {
                tab = candidate;
            }
        }
        let button = Button::new(texts.get(Text::PreviewCopy));
        if ui.add_enabled(contents.is_some(), button).clicked() {
            copy = contents.map(|contents| contents.copied(tab));
        }
    });
    ui.separator();
    body(ui, texts, browser, tab);
    if let Some(text) = copy {
        ui.ctx().copy_text(text);
    }
    model.set_preview_tab(tab);
}

fn body(ui: &mut Ui, texts: &Texts, browser: &Browser, tab: PreviewTab) {
    let Some(preview) = browser.preview() else {
        ui.weak(texts.get(Text::PreviewNone));
        return;
    };
    let entry = browser.index().entry(preview.entry());
    let path = entry.map_or("", Entry::path);
    ui.label(path);
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    match preview.status() {
        PreviewStatus::Loading => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(texts.format(Text::PreviewLoading, &[path_arg()]));
            });
        }
        PreviewStatus::TooLarge => {
            let size = entry.map_or(0, Entry::uncompressed_size);
            ui.label(texts.format(
                Text::PreviewTooLarge,
                &[
                    path_arg(),
                    ("size", format::size(size, language).into()),
                    ("limit", format::size(PREVIEW_SIZE_LIMIT, language).into()),
                ],
            ));
        }
        PreviewStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.format(Text::PreviewStopped, &[path_arg()]),
                |reason| {
                    texts.format(
                        Text::PreviewFailed,
                        &[path_arg(), ("reason", reason.to_owned().into())],
                    )
                },
            );
            ui.colored_label(ui.visuals().error_fg_color, text);
        }
        PreviewStatus::Ready(contents) => ready(ui, texts, contents, tab),
    }
}

fn ready(ui: &mut Ui, texts: &Texts, contents: &PreviewContents, tab: PreviewTab) {
    if contents.is_truncated() {
        let language = texts.language();
        let shown = u64::try_from(contents.bytes().len()).unwrap_or(u64::MAX);
        let note = texts.format(
            Text::PreviewTruncated,
            &[
                ("shown", format::number(shown, language).into()),
                ("total", format::number(contents.size(), language).into()),
            ],
        );
        ui.colored_label(ui.visuals().warn_fg_color, note);
    }
    match tab {
        PreviewTab::Text => text_rows(ui, contents),
        PreviewTab::Hex => hex_rows(ui, contents),
    }
}

fn text_rows(ui: &mut Ui, contents: &PreviewContents) {
    let height = row_height(ui);
    ScrollArea::both()
        .id_salt("preview-text")
        .auto_shrink(false)
        .show_rows(ui, height, contents.line_count(), |ui, rows| {
            for index in rows {
                ui.add(Label::new(contents.line(index)).extend());
            }
        });
}

fn hex_rows(ui: &mut Ui, contents: &PreviewContents) {
    let height = row_height(ui);
    let font = TextStyle::Monospace.resolve(ui.style());
    let layout = HexLayout::measure(ui, &font);
    ScrollArea::both()
        .id_salt("preview-hex")
        .auto_shrink(false)
        .show_rows(ui, height, contents.hex_row_count(), |ui, rows| {
            for index in rows {
                hex_row(ui, contents, index, &layout, &font);
            }
        });
}

struct HexLayout {
    offset: f32,
    byte: f32,
    character: f32,
    gap: f32,
}

impl HexLayout {
    fn measure(ui: &Ui, font: &FontId) -> Self {
        let widest = |sample: &str| {
            ui.fonts_mut(|fonts| {
                sample
                    .chars()
                    .map(|c| fonts.glyph_width(font, c))
                    .fold(0.0, f32::max)
            })
        };
        let digit = widest("0123456789ABCDEF");
        let character = widest("MW@");
        let gap = ui.spacing().item_spacing.x * 2.0;
        Self {
            offset: digit * 8.0 + gap,
            byte: digit.mul_add(2.0, gap / 2.0),
            character,
            gap,
        }
    }

    fn width(&self) -> f32 {
        let columns = f32::from(HEX_ROW_LEN);
        self.character
            .mul_add(columns, self.byte.mul_add(columns, self.offset + self.gap))
    }
}

fn hex_row(
    ui: &mut Ui,
    contents: &PreviewContents,
    index: usize,
    layout: &HexLayout,
    font: &FontId,
) {
    let height = row_height(ui);
    let (rect, response) = ui.allocate_exact_size(vec2(layout.width(), height), Sense::hover());
    let label = contents.hex_row(index);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let color = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let y = rect.center().y;
    let left = rect.left();
    let offset = label.get(..8).unwrap_or_default();
    painter.text(
        pos2(left, y),
        Align2::LEFT_CENTER,
        offset,
        font.clone(),
        weak,
    );
    let bytes = contents.hex_row_bytes(index);
    let ascii_left = layout
        .byte
        .mul_add(f32::from(HEX_ROW_LEN), left + layout.offset)
        + layout.gap;
    for (&byte, column) in bytes.iter().zip(0_u8..) {
        let column = f32::from(column);
        let x = layout.byte.mul_add(column, left + layout.offset);
        let pair = format!("{byte:02X}");
        painter.text(pos2(x, y), Align2::LEFT_CENTER, pair, font.clone(), color);
        let cell = Rect::from_min_size(
            pos2(layout.character.mul_add(column, ascii_left), rect.top()),
            vec2(layout.character, height),
        );
        let character = printable(byte).to_string();
        painter.text(
            cell.center(),
            Align2::CENTER_CENTER,
            character,
            font.clone(),
            color,
        );
    }
}
