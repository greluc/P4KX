use std::collections::HashMap;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, CursorIcon, Galley, Id, Rect, Role, ScrollArea, Sense, Stroke, Ui,
    UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};
use fluent_bundle::FluentValue;

use super::record_preview::RecordStep;
use super::table::head_fill;
use crate::theme::desktop::{
    CELL_PADDING_X, FOCUS_OFFSET, FOCUS_WIDTH, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE,
    TABLE_HEAD_TRACKING, TREE_INDENT, TREE_RAIL_OFFSET, ZEBRA,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, Weight};
use crate::widgets::button::focus_ring;
use crate::widgets::rows::paint_divider;
use crate::widgets::text::{caps, line, paint_left, plain};
use crate::{FieldLine, LineValue, RecordChangeText, RecordText, RecordView, Text, Texts, format};

const MAX_ROWS: usize = 512;
const HEADS: [Text; 2] = [
    Text::RecordChange(RecordChangeText::HeadField),
    Text::Record(RecordText::HeadValue),
];

/// Returns the value of `line` as the Record preview shows it, with its colour: a Reference in
/// the link colour, an error value in the danger tint with its reason.
pub(super) fn value_text(texts: &Texts, value: &LineValue) -> (String, Color32) {
    let error = |reason: &str| {
        let reason = ("reason", FluentValue::from(reason.to_owned()));
        (
            texts.format(RecordChangeText::ValueError, &[reason]),
            palette::DANGER_TEXT,
        )
    };
    match value {
        LineValue::Scalar(text) => (text.to_string(), palette::GRAY_1),
        LineValue::Struct(type_name) => (
            texts.format(
                RecordChangeText::ValueStruct,
                &[("type", type_name.to_string().into())],
            ),
            palette::GRAY_1,
        ),
        LineValue::Array(len) => {
            let count = u64::try_from(*len).unwrap_or(u64::MAX);
            let args = [
                ("count", FluentValue::from(count)),
                ("number", format::number(count, texts.language()).into()),
            ];
            (
                texts.format(RecordChangeText::ValueArray, &args),
                palette::GRAY_1,
            )
        }
        LineValue::Null => (texts.get(RecordChangeText::ValueNull), palette::GRAY_2_TEXT),
        LineValue::Reference(link) => {
            let text = link.at().map_or_else(
                || link.record_path().to_owned(),
                |at| {
                    texts.format(
                        RecordChangeText::ValueReferenceAt,
                        &[
                            ("path", link.record_path().to_owned().into()),
                            ("at", at.to_owned().into()),
                        ],
                    )
                },
            );
            (text, palette::PRIMARY)
        }
        LineValue::Dangling { reason, .. } | LineValue::Error(reason) => error(reason),
    }
}

/// The laid-out cells of one line.
#[derive(Clone)]
struct Laid {
    width: f32,
    name: Arc<Galley>,
    value: (Arc<Galley>, Color32),
    type_name: Option<Arc<Galley>>,
}

/// The laid-out heads and the lines shown recently, kept until the Record, the language or the
/// scale change.
#[derive(Clone, Default)]
struct LineCache {
    key: Option<Id>,
    heads: Vec<Arc<Galley>>,
    head_names: String,
    rows: HashMap<usize, Laid>,
}

impl LineCache {
    fn load(ui: &Ui, texts: &Texts, generation: u64) -> Self {
        let key = Id::unique((
            "record-lines",
            texts.language(),
            generation,
            ui.ctx().pixels_per_point().to_bits(),
        ));
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(cache_id())));
        if cache.key != Some(key) {
            let names = HEADS.map(|text| texts.get(text));
            cache = Self {
                key: Some(key),
                heads: names
                    .iter()
                    .map(|name| {
                        let text = caps(name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
                        line(ui, text, f32::INFINITY)
                    })
                    .collect(),
                head_names: names.join(" "),
                rows: HashMap::new(),
            };
        }
        cache
    }

    fn store(self, ui: &Ui) {
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(cache_id()) = self);
    }

    fn row(
        &mut self,
        ui: &Ui,
        texts: &Texts,
        (index, field): (usize, &FieldLine),
        width: f32,
    ) -> Laid {
        if let Some(kept) = self.rows.get(&index)
            && (kept.width - width).abs() < 0.5
        {
            return kept.clone();
        }
        if self.rows.len() >= MAX_ROWS {
            self.rows.clear();
        }
        let indent = f32::from(field.depth()) * TREE_INDENT;
        let name = line(
            ui,
            plain(field.name(), ROW_FONT, Weight::Regular),
            (width - indent).max(0.0),
        );
        let (text, colour) = value_text(texts, field.value());
        let linked = matches!(
            field.value(),
            LineValue::Reference(_) | LineValue::Dangling { .. }
        );
        let value = if linked {
            line(ui, plain(text, FS_XS, Weight::Bold), width)
        } else {
            line(ui, plain(text, ROW_FONT, Weight::Regular), width)
        };
        let type_name = match field.value() {
            LineValue::Reference(link) => {
                let room = (width - value.size().x - SPACE_2).max(0.0);
                Some(line(
                    ui,
                    plain(link.type_name(), ROW_FONT, Weight::Regular),
                    room,
                ))
            }
            _ => None,
        };
        let laid = Laid {
            width,
            name,
            value: (value, colour),
            type_name,
        };
        self.rows.insert(index, laid.clone());
        laid
    }
}

fn cache_id() -> Id {
    Id::unique("record-preview-lines")
}

/// Shows the head and the virtual lines of `view`, two equal columns, field and value; returns
/// the Reference that was followed.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    view: &RecordView,
    generation: u64,
) -> Option<RecordStep> {
    let mut cache = LineCache::load(ui, texts, generation);
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    heads(ui, &cache);
    let mut step = None;
    ScrollArea::vertical()
        .id_salt("record-preview-lines")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, view.lines().len(), |ui, range| {
            for index in range {
                if let Some(field) = view.lines().get(index)
                    && let Some(followed) = row(ui, texts, &mut cache, (index, field))
                {
                    step = Some(followed);
                }
            }
        });
    cache.store(ui);
    step
}

fn column_width(rect: Rect) -> f32 {
    (rect.width() / 2.0).floor()
}

fn heads(ui: &mut Ui, cache: &LineCache) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::RowHeader, true, &cache.head_names));
    head_fill(&ui.new_child(UiBuilder::new().max_rect(rect)), rect.left());
    let width = column_width(rect);
    for (index, head) in (0_u8..).zip(&cache.heads) {
        let left = f32::from(index).mul_add(width, rect.left());
        let at = pos2(left + CELL_PADDING_X, rect.center().y - head.size().y / 2.0);
        ui.painter().galley(at, Arc::clone(head), palette::GRAY_1);
    }
}

fn row(
    ui: &mut Ui,
    texts: &Texts,
    cache: &mut LineCache,
    (index, field): (usize, &FieldLine),
) -> Option<RecordStep> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let width = column_width(rect);
    let laid = cache.row(
        ui,
        texts,
        (index, field),
        2.0f32.mul_add(-CELL_PADDING_X, width),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette::GRAY_3);
    } else if index % 2 == 1 {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, ZEBRA);
    }
    paint_divider(ui.painter(), rect);
    let name_cell = Rect::from_min_max(rect.min, pos2(rect.left() + width, rect.bottom()))
        .shrink2(vec2(CELL_PADDING_X, 0.0));
    name(ui, name_cell, field.depth(), &laid.name);
    let value_cell = Rect::from_min_max(pos2(rect.left() + width, rect.top()), rect.max)
        .shrink2(vec2(CELL_PADDING_X, 0.0));
    let described = || {
        texts.format(
            RecordText::Row,
            &[
                ("field", field.name().to_owned().into()),
                ("value", value_text(texts, field.value()).0.into()),
            ],
        )
    };
    response.widget_info(|| WidgetInfo::labeled(Role::Row, true, described()));
    response.on_hover_ui(|ui| {
        ui.label(described());
    });
    match field.value() {
        LineValue::Reference(link) => {
            let label = || {
                texts.format(
                    RecordText::Follow,
                    &[("path", link.record_path().to_owned().into())],
                )
            };
            let clicked = link_cell(ui, (index, value_cell), &laid, label);
            clicked.then(|| RecordStep::Follow(link.clone()))
        }
        LineValue::Dangling { guid, .. } => {
            let label = || value_text(texts, field.value()).0;
            let clicked = link_cell(ui, (index, value_cell), &laid, label);
            clicked.then_some(RecordStep::Dangling(*guid))
        }
        _ => {
            let (galley, colour) = &laid.value;
            paint_left(ui, value_cell, Arc::clone(galley), *colour);
            None
        }
    }
}

/// Paints the depth rails and the name of a line of `depth` in `cell`.
fn name(ui: &Ui, cell: Rect, depth: u16, galley: &Arc<Galley>) {
    for level in 0..depth {
        let x = f32::from(level).mul_add(TREE_INDENT, cell.left() + TREE_RAIL_OFFSET);
        ui.painter()
            .vline(x, cell.y_range(), Stroke::new(1.0, palette::GRAY_3));
    }
    let indent = f32::from(depth) * TREE_INDENT;
    let text = cell.with_min_x((cell.left() + indent).min(cell.right()));
    paint_left(ui, text, Arc::clone(galley), palette::GRAY_1);
}

/// Shows the value of a link line in `cell` as a link named by `label`, the target's type after it;
/// returns whether it was clicked.
fn link_cell(
    ui: &Ui,
    (index, cell): (usize, Rect),
    laid: &Laid,
    label: impl Fn() -> String,
) -> bool {
    let (galley, colour) = &laid.value;
    let size = vec2(galley.size().x.min(cell.width()), galley.size().y);
    let link = Rect::from_min_size(pos2(cell.left(), cell.center().y - size.y / 2.0), size);
    let response = ui
        .interact(
            link,
            ui.scope_id().with(("record-link", index)),
            Sense::click(),
        )
        .on_hover_cursor(CursorIcon::PointingHand);
    response.widget_info(|| WidgetInfo::labeled(Role::Link, true, label()));
    let hovered = response.hovered();
    let shown = if hovered && *colour == palette::PRIMARY {
        palette::ACCENT_LIGHT
    } else {
        *colour
    };
    ui.painter().galley(link.min, Arc::clone(galley), shown);
    if hovered {
        ui.painter().hline(
            link.x_range(),
            link.bottom(),
            Stroke::new(FOCUS_WIDTH / 2.0, shown),
        );
    }
    if response.has_focus() {
        focus_ring(ui, link, palette::PRIMARY, FOCUS_OFFSET);
    }
    if let Some(type_name) = &laid.type_name {
        let after = cell.with_min_x((link.right() + SPACE_2).min(cell.right()));
        paint_left(ui, after, Arc::clone(type_name), palette::GRAY_2_TEXT);
    }
    response.clicked()
}
