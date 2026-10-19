use std::sync::Arc;

use egui::{
    Align2, Color32, Galley, Id, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2,
};

use crate::preview::{HEX_ROW_LEN, printable};
use crate::theme::desktop::{
    HEX_ASCII_GAP, HEX_ASCII_WIDTH, HEX_BYTE_GAP, HEX_BYTE_WIDTH, HEX_GROUP_GAP, HEX_OFFSET_WIDTH,
    HEX_ROW_HEIGHT,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight, font};
use crate::widgets::scroll_region;
use crate::{PreviewContents, ScrollText, Texts};

const GROUP: u8 = 8;
/// Shows the hex view of contents: offsets, bytes and printable characters, row by row.
pub(super) fn hex_rows(ui: &mut Ui, texts: &Texts, contents: &PreviewContents) {
    let glyphs = HexGlyphs::get(ui);
    let area = ScrollArea::both().id_salt("preview-hex").auto_shrink(false);
    let id = ui.scope_id().with("preview-hex");
    scroll_region::show(
        ui,
        (id, || texts.get(ScrollText::Hex)),
        HEX_ROW_HEIGHT + ui.spacing().item_spacing.y,
        area,
        |area, ui| {
            area.show_rows(ui, HEX_ROW_HEIGHT, contents.hex_row_count(), |ui, rows| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for index in rows {
                    hex_row(ui, contents, index, &glyphs);
                }
            })
        },
    );
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
    let offset = u32::try_from(index.saturating_mul(usize::from(HEX_ROW_LEN))).unwrap_or(u32::MAX);
    let mut left = rect.left();
    for byte in offset.to_be_bytes() {
        if let Some(pair) = glyphs.pairs.get(usize::from(byte)) {
            let at = pos2(left, y - pair.size().y / 2.0);
            left += pair.size().x;
            painter.galley(at, Arc::clone(pair), palette::GRAY_2_TEXT);
        }
    }
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
