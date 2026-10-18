use std::sync::Arc;

use egui::{
    Align2, Color32, CornerRadius, Galley, Id, Rect, Response, Role, ScrollArea, Sense, Ui,
    WidgetInfo, pos2, vec2,
};
use fluent_bundle::FluentValue;

use crate::theme::desktop::{
    CELL_PADDING_X, DETAIL_COLUMN_GAP, DETAIL_KEY_MIN_WIDTH, DETAIL_KEY_TRACKING, DETAIL_ROW_GAP,
    DETAIL_VALUE_SIZE, PANEL_TITLE_MARGIN_BOTTOM, PANEL_TITLE_MARGIN_TOP,
    PANEL_TITLE_PADDING_BOTTOM, PANEL_TITLE_SIZE, ROW_FONT, ROW_HEIGHT, SPINNER_INLINE,
    TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING, ZEBRA,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, TRACKING_DISPLAY, Weight};
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::widgets::rows::paint_divider;
use crate::widgets::text::{caps, hairline_below, line, plain};
use crate::{
    DataForgeFailure, DataForgeSummary, DataForgeText, Language, RecordTree, RecordTreeStatus,
    Text, Texts, format,
};

const ROWS: u8 = 3;

const COLUMNS: [Text; 3] = [
    Text::ColumnName,
    Text::DataForge(DataForgeText::ColumnType),
    Text::DataForge(DataForgeText::ColumnRecordPath),
];

/// Returns the message that explains a DataForge `status` and its colour, or `None` while it
/// loads or once it is loaded.
pub(super) fn message(
    texts: &Texts,
    path: &str,
    status: RecordTreeStatus<'_>,
) -> Option<(String, Color32)> {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let failed = |text: String| Some((text, palette::DANGER_TEXT));
    match status {
        RecordTreeStatus::Failed(DataForgeFailure::Unsupported { version }) => {
            failed(texts.format(
                DataForgeText::Unsupported,
                &[("version", (*version).into())],
            ))
        }
        RecordTreeStatus::Failed(DataForgeFailure::Unreadable(reason)) => failed(texts.format(
            DataForgeText::Failed,
            &[path_arg(), ("reason", reason.clone().into())],
        )),
        RecordTreeStatus::Failed(DataForgeFailure::Stopped) => {
            failed(texts.format(DataForgeText::Stopped, &[path_arg()]))
        }
        RecordTreeStatus::ArchiveChanged => {
            Some((texts.get(DataForgeText::ArchiveChanged), palette::WARNING))
        }
        RecordTreeStatus::NotLoaded | RecordTreeStatus::Loading | RecordTreeStatus::Ready(_) => {
            None
        }
    }
}

/// Shows the body of the DataForge tab for `tree`.
pub(super) fn show(ui: &mut Ui, texts: &Texts, tree: &RecordTree) {
    frames::preview_body().show(ui, |ui| match tree.status() {
        RecordTreeStatus::NotLoaded | RecordTreeStatus::Loading => {
            ui.horizontal(|ui| {
                spinner(ui, SPINNER_INLINE);
                ui.label(texts.format(
                    DataForgeText::Loading,
                    &[("path", tree.entry_path().to_owned().into())],
                ));
            });
        }
        RecordTreeStatus::Ready(summary) => {
            let laid = Laid::get(ui, texts, tree.generation(), summary);
            ScrollArea::vertical()
                .id_salt("dataforge-summary")
                .auto_shrink(false)
                .show(ui, |ui| laid.show(ui));
        }
        status => {
            if let Some((text, colour)) = message(texts, tree.entry_path(), status) {
                ui.colored_label(colour, text);
            }
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Key {
    generation: u64,
    language: Language,
    width: f32,
    pixels_per_point: f32,
}

type Labelled = (String, Arc<Galley>);

/// The laid-out summary, kept across frames until the Records, the language or the width change.
struct Laid {
    key: Key,
    facts: Vec<(Labelled, Labelled)>,
    heading: Labelled,
    head: Vec<Labelled>,
    rows: Vec<Vec<Labelled>>,
}

impl Laid {
    fn get(ui: &Ui, texts: &Texts, generation: u64, summary: &DataForgeSummary) -> Arc<Self> {
        let id = Id::unique("p4kx-dataforge-summary");
        let key = Key {
            generation,
            language: texts.language(),
            width: ui.available_width(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        if let Some(laid) = ui.data(|data| data.get_temp::<Arc<Self>>(id))
            && laid.key == key
        {
            return laid;
        }
        let laid = Arc::new(Self::lay_out(ui, texts, key, summary));
        ui.data_mut(|data| data.insert_temp(id, Arc::clone(&laid)));
        laid
    }

    fn lay_out(ui: &Ui, texts: &Texts, key: Key, summary: &DataForgeSummary) -> Self {
        let facts = facts(ui, texts, key.width, summary);
        let heading_text = texts.get(DataForgeText::FirstRecords);
        let heading = line(
            ui,
            caps(
                &heading_text,
                PANEL_TITLE_SIZE,
                Weight::Bold,
                TRACKING_DISPLAY,
            ),
            key.width,
        );
        let cell_width = 2.0f32.mul_add(-CELL_PADDING_X, key.width / 3.0).max(0.0);
        let head = COLUMNS
            .iter()
            .map(|&column| {
                let name = texts.get(column);
                let text = caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
                (name, line(ui, text, cell_width))
            })
            .collect();
        let cell = |text: &str| {
            (
                text.to_owned(),
                line(ui, plain(text, ROW_FONT, Weight::Regular), cell_width),
            )
        };
        let rows = summary
            .first_records()
            .iter()
            .map(|record| {
                vec![
                    cell(record.name()),
                    cell(record.type_name()),
                    cell(record.record_path()),
                ]
            })
            .collect();
        Self {
            key,
            facts,
            heading: (heading_text, heading),
            head,
            rows,
        }
    }

    fn show(&self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 0.0;
        let key_width = self
            .facts
            .iter()
            .map(|((_, galley), _)| galley.size().x)
            .fold(DETAIL_KEY_MIN_WIDTH, f32::max);
        for (index, (key, value)) in self.facts.iter().enumerate() {
            if index > 0 {
                ui.add_space(DETAIL_ROW_GAP);
            }
            let height = key.1.size().y.max(value.1.size().y) + DETAIL_ROW_GAP;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let content = rect.with_max_y(rect.bottom() - DETAIL_ROW_GAP);
            let key_rect = rect.with_max_x(rect.left() + key_width);
            let value_rect = rect.with_min_x(key_rect.right() + DETAIL_COLUMN_GAP);
            let on_baseline = |cell: Rect, galley: &Galley| {
                cell.with_min_y(content.bottom() - galley.size().y)
                    .with_max_y(content.bottom())
            };
            labelled(
                ui,
                (0, index, 0),
                key,
                (on_baseline(key_rect, &key.1), palette::GRAY_2_TEXT),
            );
            labelled(
                ui,
                (0, index, 1),
                value,
                (on_baseline(value_rect, &value.1), palette::DATA_FG),
            );
            hairline_below(ui, key_rect);
            hairline_below(ui, value_rect);
        }
        ui.add_space(PANEL_TITLE_MARGIN_TOP);
        let size = vec2(
            ui.available_width(),
            self.heading.1.size().y + PANEL_TITLE_PADDING_BOTTOM,
        );
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let text_rect = rect.with_max_y(rect.bottom() - PANEL_TITLE_PADDING_BOTTOM);
        labelled(ui, (1, 0, 0), &self.heading, (text_rect, palette::GRAY_1));
        hairline_below(ui, rect);
        ui.add_space(PANEL_TITLE_MARGIN_BOTTOM);
        self.table(ui);
    }

    fn table(&self, ui: &mut Ui) {
        let (head, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(head, CornerRadius::ZERO, palette::SURFACE_INPUT);
        let rule = Rect::from_min_max(pos2(head.left(), head.bottom() - 2.0), head.max);
        painter.rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
        cells(ui, (2, 0), head, &self.head);
        for (index, row) in self.rows.iter().enumerate() {
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
            if ui.rect_contains_pointer(rect) {
                ui.painter()
                    .rect_filled(rect, CornerRadius::ZERO, palette::GRAY_3);
            } else if index % 2 == 1 {
                ui.painter().rect_filled(rect, CornerRadius::ZERO, ZEBRA);
            }
            paint_divider(ui.painter(), rect);
            cells(ui, (ROWS, index), rect, row);
        }
    }
}

fn cells(ui: &Ui, (section, row): (u8, usize), rect: Rect, cells: &[Labelled]) {
    let width = rect.width() / 3.0;
    for ((column, step), cell) in (0_usize..).zip(0_u8..).zip(cells) {
        let left = f32::from(step).mul_add(width, rect.left());
        let cell_rect = Rect::from_min_size(pos2(left, rect.top()), vec2(width, rect.height()))
            .shrink2(vec2(CELL_PADDING_X, 0.0));
        let response = labelled(
            ui,
            (section, row, column),
            cell,
            (cell_rect, palette::GRAY_1),
        );
        if section == ROWS {
            response.on_hover_ui(|ui| {
                ui.label(cell.0.as_str());
            });
        }
    }
}

fn labelled(
    ui: &Ui,
    salt: (u8, usize, usize),
    (name, galley): &Labelled,
    (rect, colour): (Rect, Color32),
) -> Response {
    let at = Align2::LEFT_CENTER
        .align_size_within_rect(galley.size(), rect)
        .min;
    let response = ui.interact(
        rect,
        ui.scope_id().with(("dataforge-text", salt)),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, name));
    ui.painter().galley(at, Arc::clone(galley), colour);
    response
}

fn facts(
    ui: &Ui,
    texts: &Texts,
    width: f32,
    summary: &DataForgeSummary,
) -> Vec<(Labelled, Labelled)> {
    let language = texts.language();
    let count = |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), language);
    [
        (DataForgeText::Version, summary.version().to_string()),
        (DataForgeText::Records, count(summary.record_count())),
        (DataForgeText::Structs, count(summary.struct_count())),
        (DataForgeText::Enums, count(summary.enum_count())),
    ]
    .into_iter()
    .map(|(name, value)| {
        let name = texts.get(name);
        let key_text = caps(&name, FS_SM, Weight::Bold, DETAIL_KEY_TRACKING);
        let value_text = plain(value.clone(), DETAIL_VALUE_SIZE, Weight::Light);
        (
            (name, line(ui, key_text, f32::INFINITY)),
            (value, line(ui, value_text, width)),
        )
    })
    .collect()
}
