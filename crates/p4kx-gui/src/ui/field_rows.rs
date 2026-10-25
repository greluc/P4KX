// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, Galley, Id, Rect, Role, ScrollArea, Sense, Ui, UiBuilder, Vec2,
    WidgetInfo, pos2, vec2,
};

use super::table::head_fill;
use fluent_bundle::FluentValue;

use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING, ZEBRA,
};
use crate::theme::palette;
use crate::theme::tokens::Weight;
use crate::widgets::rows::{focus_inside, paint_divider};
use crate::widgets::scroll_region;
use crate::widgets::text::{caps, line, paint_left, plain};
use crate::widgets::tooltip;
use crate::{
    DiffText, FieldName, FieldRow, FieldValue, RecordChangeText, RecordFields, ScrollText, Text,
    Texts, format,
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
        FieldName::RecordTag => texts.get(RecordChangeText::RecordTag),
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
    let area = ScrollArea::vertical()
        .id_salt("comparison-field-rows")
        .auto_shrink(false);
    let id = ui.scope_id().with("comparison-field-rows");
    scroll_region::show(
        ui,
        (id, || texts.get(ScrollText::Fields)),
        ROW_HEIGHT,
        area,
        |area, ui| {
            area.show_rows(ui, ROW_HEIGHT, fields.rows().len(), |ui, range| {
                for index in range {
                    if let Some(row) = fields.rows().get(index) {
                        field_row(ui, texts, &mut cache, (index, row));
                    }
                }
            })
        },
    );
    cache.store(ui);
}

fn column_width(rect: Rect) -> f32 {
    (rect.width() / 3.0).floor()
}

fn heads(ui: &mut Ui, cache: &FieldCache) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::RowHeader, true, &cache.head_names));
    head_fill(&ui.new_child(UiBuilder::new().max_rect(rect)), rect.left());
    let painter = ui.painter();
    let width = column_width(rect);
    for (index, head) in (0_u8..).zip(&cache.heads) {
        let left = f32::from(index).mul_add(width, rect.left());
        let at = pos2(left + CELL_PADDING_X, rect.center().y - head.size().y / 2.0);
        painter.galley(at, Arc::clone(head), palette::GRAY_1);
    }
}

fn field_row(ui: &mut Ui, texts: &Texts, cache: &mut FieldCache, (index, row): (usize, &FieldRow)) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let width = column_width(rect);
    let laid = cache.row(
        ui,
        texts,
        (index, row),
        2.0f32.mul_add(-CELL_PADDING_X, width),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette::CONTROL_BG);
    } else if index % 2 == 1 {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, ZEBRA);
    }
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
    focus_inside(ui.painter(), rect, &response);
    if response.has_focus() {
        tooltip::show(&response, |ui| {
            tooltip::line(ui, described());
        });
    } else {
        tooltip::hover(response, |ui| {
            tooltip::line(ui, described());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::value_text;
    use crate::theme::palette;
    use crate::{FieldValue, Language, Texts};

    #[test]
    fn values_read_as_words_in_both_languages() {
        let english = Texts::new(Language::BritishEnglish);
        let german = Texts::new(Language::German);
        let text = |texts: &Texts, value: Option<FieldValue>| value_text(texts, value.as_ref());

        assert_eq!(
            text(&english, Some(FieldValue::Struct("Hull".into()))),
            ("Hull (struct)".to_owned(), palette::GRAY_1),
            "struct"
        );
        assert_eq!(
            text(&english, Some(FieldValue::Array(1))),
            ("1 element".to_owned(), palette::GRAY_1),
            "one element"
        );
        assert_eq!(
            text(&german, Some(FieldValue::Array(3))),
            ("3 Elemente".to_owned(), palette::GRAY_1),
            "three elements in German"
        );
        assert_eq!(
            text(&english, Some(FieldValue::Null)),
            ("null".to_owned(), palette::GRAY_1),
            "null"
        );
        assert_eq!(
            text(&english, None),
            ("–".to_owned(), palette::GRAY_2_TEXT),
            "missing, muted"
        );
        assert_eq!(
            text(&german, Some(FieldValue::Error("bad offset".into()))),
            ("Fehler: bad offset".to_owned(), palette::DANGER_TEXT),
            "error with its reason in the danger tint"
        );
    }
}
