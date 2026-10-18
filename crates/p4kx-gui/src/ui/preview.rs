use std::sync::Arc;

use egui::{
    Align, Align2, Color32, Galley, Id, Label, Layout, Rect, Role, ScrollArea, Sense, Ui,
    WidgetInfo, pos2, vec2,
};
use fluent_bundle::FluentValue;
use p4kx_extraction::Entry;

use crate::preview::{HEX_ROW_LEN, printable};
use crate::theme::desktop::{
    HEX_ASCII_GAP, HEX_ASCII_WIDTH, HEX_BYTE_GAP, HEX_BYTE_WIDTH, HEX_GROUP_GAP, HEX_OFFSET_WIDTH,
    HEX_ROW_HEIGHT, PREVIEW_PADDING_X, PREVIEW_PATH_HEIGHT,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, TOUCH_TARGET, Weight, font};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames;
use crate::widgets::tab;
use crate::widgets::text::{hairline_below, line, paint_left, plain};
use crate::{
    Browser, Model, PREVIEW_SIZE_LIMIT, PreviewContents, PreviewStatus, PreviewTab, Text, Texts,
    format,
};

const TABS: [(PreviewTab, Text); 2] = [
    (PreviewTab::Text, Text::PreviewTabText),
    (PreviewTab::Hex, Text::PreviewTabHex),
];
const GROUP: u8 = 8;

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let mut tab = model.preview_tab();
    let contents = browser.preview().and_then(|preview| preview.contents());
    let mut copy = None;
    let labels = TABS.map(|(_, text)| texts.get(text));
    let current = TABS
        .iter()
        .position(|(candidate, _)| *candidate == tab)
        .unwrap_or(0);
    let bar = ui.max_rect().with_max_y(ui.max_rect().top() + TOUCH_TARGET);
    if let Some(index) = tab::tab_bar(ui, &labels, current)
        && let Some((candidate, _)) = TABS.get(index)
    {
        tab = *candidate;
    }
    let copy_label = texts.get(Text::PreviewCopy);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(bar.shrink2(vec2(SPACE_2, 0.0))),
        |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let enabled = contents.is_some();
                if button::button(ui, Kind::Ghost, Size::Compact, &copy_label, enabled).clicked() {
                    copy = contents.map(|contents| contents.copied(tab));
                }
            });
        },
    );
    body(ui, texts, browser, tab);
    if let Some(text) = copy {
        ui.ctx().copy_text(text);
    }
    model.set_preview_tab(tab);
}

fn body(ui: &mut Ui, texts: &Texts, browser: &Browser, tab: PreviewTab) {
    let Some(preview) = browser.preview() else {
        frames::preview_body().show(ui, |ui| {
            ui.label(
                plain(texts.get(Text::PreviewNone), FS_XS, Weight::Regular)
                    .color(palette::GRAY_2_TEXT),
            );
        });
        return;
    };
    let entry = browser.index().entry(preview.entry());
    let path = entry.map_or("", Entry::path);
    let size = entry.map_or(0, Entry::uncompressed_size);
    path_line(ui, path, &format::size(size, texts.language()));
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    frames::preview_body().show(ui, |ui| match preview.status() {
        PreviewStatus::Loading => {
            ui.horizontal(|ui| {
                crate::widgets::indicator::spinner(ui, crate::theme::desktop::SPINNER_INLINE);
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
            ui.colored_label(palette::DANGER_TEXT, text);
        }
        PreviewStatus::ArchiveChanged => {
            ui.colored_label(palette::WARNING, texts.get(Text::PreviewArchiveChanged));
        }
        PreviewStatus::Ready(contents) => ready(ui, texts, contents, tab),
    });
}

fn path_line(ui: &mut Ui, path: &str, size: &str) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), PREVIEW_PATH_HEIGHT),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, path));
    let inner = rect.shrink2(vec2(PREVIEW_PADDING_X, 0.0));
    let size = line(ui, plain(size, FS_XS, Weight::Regular), f32::INFINITY);
    let galley = line(
        ui,
        plain(path, FS_XS, Weight::Regular),
        inner.width() - size.size().x - SPACE_2,
    );
    let size_rect = inner.with_min_x(inner.left() + galley.size().x + SPACE_2);
    paint_left(ui, inner, galley, palette::GRAY_1);
    paint_left(ui, size_rect, size, palette::GRAY_2_TEXT);
    hairline_below(ui, rect);
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
        ui.colored_label(palette::WARNING, note);
    }
    match tab {
        PreviewTab::Text => text_rows(ui, contents),
        PreviewTab::Hex => hex_rows(ui, contents),
    }
}

fn text_rows(ui: &mut Ui, contents: &PreviewContents) {
    ScrollArea::both()
        .id_salt("preview-text")
        .auto_shrink(false)
        .show_rows(ui, HEX_ROW_HEIGHT, contents.line_count(), |ui, rows| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for index in rows {
                ui.add(Label::new(contents.line(index)).extend());
            }
        });
}

fn hex_rows(ui: &mut Ui, contents: &PreviewContents) {
    let glyphs = HexGlyphs::get(ui);
    ScrollArea::both()
        .id_salt("preview-hex")
        .auto_shrink(false)
        .show_rows(ui, HEX_ROW_HEIGHT, contents.hex_row_count(), |ui, rows| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for index in rows {
                hex_row(ui, contents, index, &glyphs);
            }
        });
}

/// The laid-out byte pairs and characters of the hex view, kept across frames per scale.
#[derive(Clone)]
struct HexGlyphs {
    pixels_per_point: f32,
    pairs: Arc<[Arc<Galley>]>,
    characters: Arc<[Arc<Galley>]>,
}

impl HexGlyphs {
    fn get(ui: &Ui) -> Self {
        let id = Id::unique("p4kx-hex-glyphs");
        let pixels_per_point = ui.ctx().pixels_per_point();
        if let Some(cached) = ui.data(|data| data.get_temp::<Self>(id))
            && (cached.pixels_per_point - pixels_per_point).abs() < f32::EPSILON
        {
            return cached;
        }
        let font = font(FS_XS, Weight::Regular);
        let layout = |text: String| {
            ui.fonts_mut(|fonts| fonts.layout_no_wrap(text, font.clone(), Color32::PLACEHOLDER))
        };
        let glyphs = Self {
            pixels_per_point,
            pairs: (0..=u8::MAX)
                .map(|byte| layout(format!("{byte:02X}")))
                .collect(),
            characters: (0..=u8::MAX)
                .map(|byte| layout(printable(byte).to_string()))
                .collect(),
        };
        ui.data_mut(|data| data.insert_temp(id, glyphs.clone()));
        glyphs
    }
}

const fn muted(byte: u8) -> bool {
    !matches!(byte, 0x21..=0x7E)
}

fn byte_left(column: u8) -> f32 {
    let groups = f32::from(column / GROUP);
    f32::from(column).mul_add(
        HEX_BYTE_WIDTH + HEX_BYTE_GAP,
        groups.mul_add(HEX_GROUP_GAP, HEX_OFFSET_WIDTH),
    )
}

fn hex_row(ui: &mut Ui, contents: &PreviewContents, index: usize, glyphs: &HexGlyphs) {
    let ascii_left = byte_left(HEX_ROW_LEN) + HEX_ASCII_GAP;
    let width = f32::from(HEX_ROW_LEN).mul_add(HEX_ASCII_WIDTH, ascii_left);
    let (rect, response) = ui.allocate_exact_size(vec2(width, HEX_ROW_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, contents.hex_row(index)));
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let y = rect.center().y;
    let offset = index.saturating_mul(usize::from(HEX_ROW_LEN));
    painter.text(
        pos2(rect.left(), y),
        Align2::LEFT_CENTER,
        format!("{offset:08X}"),
        font(FS_XS, Weight::Regular),
        palette::GRAY_2_TEXT,
    );
    for (&byte, column) in contents.hex_row_bytes(index).iter().zip(0_u8..) {
        let colour = if byte == 0 {
            palette::GRAY_2_TEXT
        } else {
            palette::GRAY_1
        };
        if let Some(pair) = glyphs.pairs.get(usize::from(byte)) {
            let cell = Rect::from_min_size(
                pos2(rect.left() + byte_left(column), rect.top()),
                vec2(HEX_BYTE_WIDTH, HEX_ROW_HEIGHT),
            );
            let at = Align2::CENTER_CENTER
                .align_size_within_rect(pair.size(), cell)
                .min;
            painter.galley(at, Arc::clone(pair), colour);
        }
        if let Some(character) = glyphs.characters.get(usize::from(byte)) {
            let cell = Rect::from_min_size(
                pos2(
                    f32::from(column).mul_add(HEX_ASCII_WIDTH, rect.left() + ascii_left),
                    rect.top(),
                ),
                vec2(HEX_ASCII_WIDTH, HEX_ROW_HEIGHT),
            );
            let at = Align2::CENTER_CENTER
                .align_size_within_rect(character.size(), cell)
                .min;
            let colour = if muted(byte) {
                palette::GRAY_2_TEXT
            } else {
                palette::GRAY_1
            };
            painter.galley(at, Arc::clone(character), colour);
        }
    }
}
