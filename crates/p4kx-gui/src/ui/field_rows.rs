use std::collections::HashMap;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, Galley, Id, Rect, Role, ScrollArea, Sense, Ui, Vec2, WidgetInfo, pos2,
    vec2,
};
use fluent_bundle::FluentValue;

use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::Weight;
use crate::widgets::rows::paint_divider;
use crate::widgets::text::{caps, line, paint_left, plain};
use crate::{
    DiffText, FieldName, FieldRow, FieldValue, RecordChangeText, RecordFields, Text, Texts, format,
};

const MAX_ROWS: usize = 512;
const HEADS: [Text; 3] = [
    Text::RecordChange(RecordChangeText::HeadField),
    Text::Diff(DiffText::HeadOld),
    Text::Diff(DiffText::HeadNew),
];

/// Returns the text of what `row` names.
pub(super) fn field_name(texts: &Texts, row: &FieldRow) -> String {
    match &row.name {
        FieldName::RecordName => texts.get(RecordChangeText::RecordName),
        FieldName::RecordType => texts.get(RecordChangeText::RecordType),
        FieldName::Field(path) => path.to_string(),
    }
}

/// Returns one side's value as the field list shows it, with its colour: a missing value
/// muted, an error value in the danger tint with its reason.
pub(super) fn value_text(texts: &Texts, value: Option<&FieldValue>) -> (String, Color32) {
    let plain_text = |text: String| (text, palette::GRAY_1);
    let Some(value) = value else {
        return (
            texts.get(RecordChangeText::ValueMissing),
            palette::GRAY_2_TEXT,
        );
    };
    match value {
        FieldValue::Scalar(text) => plain_text(text.to_string()),
        FieldValue::Reference {
            record_path,
            at: None,
        } => plain_text(record_path.to_string()),
        FieldValue::Reference {
            record_path,
            at: Some(at),
        } => plain_text(texts.format(
            RecordChangeText::ValueReferenceAt,
            &[
                ("path", record_path.to_string().into()),
                ("at", at.to_string().into()),
            ],
        )),
        FieldValue::Struct(type_name) => plain_text(texts.format(
            RecordChangeText::ValueStruct,
            &[("type", type_name.to_string().into())],
        )),
        FieldValue::Array(len) => {
            let count = u64::try_from(*len).unwrap_or(u64::MAX);
            plain_text(texts.format(
                RecordChangeText::ValueArray,
                &[
                    ("count", FluentValue::from(count)),
                    ("number", format::number(count, texts.language()).into()),
                ],
            ))
        }
        FieldValue::Null => plain_text(texts.get(RecordChangeText::ValueNull)),
        FieldValue::Error(reason) => (
            texts.format(
                RecordChangeText::ValueError,
                &[("reason", reason.to_string().into())],
            ),
            palette::DANGER_TEXT,
        ),
    }
}

/// The laid-out cells of one field row.
#[derive(Clone)]
struct Laid {
    width: f32,
    cells: [(Arc<Galley>, Color32); 3],
}

/// The laid-out heads and the field rows shown recently, kept until the detail, the language
/// or the scale change.
#[derive(Clone, Default)]
struct FieldCache {
    key: Option<Id>,
    heads: Vec<Arc<Galley>>,
    head_names: String,
    rows: HashMap<usize, Laid>,
}

impl FieldCache {
    fn load(ui: &Ui, texts: &Texts, generation: u64) -> Self {
        let key = Id::unique((
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
                        line(
                            ui,
                            caps(name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING),
                            f32::INFINITY,
                        )
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
        (index, row): (usize, &FieldRow),
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
        let cell = |(text, colour): (String, Color32)| {
            (
                line(ui, plain(text, ROW_FONT, Weight::Regular), width),
                colour,
            )
        };
        let laid = Laid {
            width,
            cells: [
                cell((field_name(texts, row), palette::GRAY_1)),
                cell(value_text(texts, row.old.as_ref())),
                cell(value_text(texts, row.new.as_ref())),
            ],
        };
        self.rows.insert(index, laid.clone());
        laid
    }
}

fn cache_id() -> Id {
    Id::unique("comparison-field-rows")
}

/// Shows the head and the virtual rows of `fields`, three equal columns: field, old, new.
pub(super) fn show(ui: &mut Ui, texts: &Texts, fields: &RecordFields, generation: u64) {
    let mut cache = FieldCache::load(ui, texts, generation);
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    heads(ui, &cache);
    ScrollArea::vertical()
        .id_salt("comparison-field-rows")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, fields.rows().len(), |ui, range| {
            for index in range {
                if let Some(row) = fields.rows().get(index) {
                    field_row(ui, texts, &mut cache, (index, row));
                }
            }
        });
    cache.store(ui);
}

fn column_width(rect: Rect) -> f32 {
    (rect.width() / 3.0).floor()
}

fn heads(ui: &mut Ui, cache: &FieldCache) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::RowHeader, true, &cache.head_names));
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
    let rule = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
    painter.rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
    let width = column_width(rect);
    for (index, head) in (0_u8..).zip(&cache.heads) {
        let left = f32::from(index).mul_add(width, rect.left());
        let at = pos2(left + CELL_PADDING_X, rect.center().y - head.size().y / 2.0);
        painter.galley(at, Arc::clone(head), palette::GRAY_1);
    }
}

fn field_row(ui: &mut Ui, texts: &Texts, cache: &mut FieldCache, (index, row): (usize, &FieldRow)) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let width = column_width(rect);
    let laid = cache.row(
        ui,
        texts,
        (index, row),
        2.0f32.mul_add(-CELL_PADDING_X, width),
    );
    paint_divider(ui.painter(), rect);
    for (column, (galley, colour)) in (0_u8..).zip(laid.cells) {
        let left = f32::from(column).mul_add(width, rect.left());
        let cell = Rect::from_min_max(pos2(left, rect.top()), pos2(left + width, rect.bottom()))
            .shrink2(vec2(CELL_PADDING_X, 0.0));
        paint_left(ui, cell, galley, colour);
    }
    let described = || {
        texts.format(
            RecordChangeText::FieldRow,
            &[
                ("field", field_name(texts, row).into()),
                ("old", value_text(texts, row.old.as_ref()).0.into()),
                ("new", value_text(texts, row.new.as_ref()).0.into()),
            ],
        )
    };
    response.widget_info(|| WidgetInfo::labeled(Role::Row, true, described()));
    response.on_hover_ui(|ui| {
        ui.label(described());
    });
}
