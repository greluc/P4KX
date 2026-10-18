use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use egui::{
    Color32, CornerRadius, Galley, Id, Rect, Response, Role, ScrollArea, Sense, Ui, Vec2,
    WidgetInfo, pos2, vec2,
};

use super::record_tree::status_code;
use super::search::eyebrow;
use crate::texts::RecordSearchText;
use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING, ZEBRA,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_row, row_weight};
use crate::widgets::text::{caps, line, paint_left, plain};
use crate::{
    Browser, DataForgeText, Language, RecordId, RecordInfo, RecordNodeKind, RecordTree, Text, Texts,
};

const MAX_CACHED: usize = 4096;

const COLUMNS: [Text; 3] = [
    Text::ColumnName,
    Text::DataForge(DataForgeText::ColumnType),
    Text::DataForge(DataForgeText::ColumnRecordPath),
];

#[derive(Debug, Clone, Copy, PartialEq)]
struct Key {
    matches: u64,
    records: u64,
    status: u8,
    language: Language,
    width: f32,
    pixels_per_point: f32,
}

/// A line of text the Record search shows besides the matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Note {
    /// The search runs.
    Searching,
    /// The number of matches.
    Count,
    /// Asks for text while the field is empty.
    Prompt,
    /// Nothing matched.
    NoMatches,
    /// Where loading DataForge stands.
    Status,
}

type Labelled = (String, Arc<Galley>);

#[derive(Default)]
struct Cache {
    key: Option<Key>,
    label: Option<Labelled>,
    hint: Option<Arc<str>>,
    heads: Vec<Labelled>,
    cells: HashMap<(RecordId, u8, bool), Arc<Galley>>,
    notes: HashMap<Note, Arc<Galley>>,
    text: String,
}

/// The laid-out texts of the Record search, kept across frames until the matches, the loaded
/// Records, the language or the width change; it also holds the text of the search field.
#[derive(Clone)]
pub(super) struct Laid(Arc<Mutex<Cache>>);

impl Laid {
    pub(super) fn get(ui: &Ui, texts: &Texts, browser: &Browser, tree: &RecordTree) -> Self {
        let id = Id::unique("p4kx-record-search");
        let laid = ui.data_mut(|data| {
            data.get_temp_mut_or_insert_with(id, || Self(Arc::new(Mutex::new(Cache::default()))))
                .clone()
        });
        let key = Key {
            matches: browser.record_search_generation(),
            records: tree.generation(),
            status: status_code(tree.status()),
            language: texts.language(),
            width: ui.available_width(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        let mut cache = laid.lock();
        if cache.key != Some(key) || cache.cells.len() > MAX_CACHED {
            let text = std::mem::take(&mut cache.text);
            *cache = Cache {
                key: Some(key),
                label: Some(label(ui, texts)),
                hint: Some(texts.get(RecordSearchText::Hint).into()),
                heads: heads(ui, texts, cell_width(key.width)),
                cells: HashMap::new(),
                notes: HashMap::new(),
                text,
            };
        }
        drop(cache);
        laid
    }

    fn lock(&self) -> MutexGuard<'_, Cache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Shows the label of the search field, named by its Fluent text.
    pub(super) fn show_label(&self, ui: &mut Ui) -> Response {
        let cache = self.lock();
        let Some((name, galley)) = &cache.label else {
            return ui.allocate_response(Vec2::ZERO, Sense::hover());
        };
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, name));
        ui.painter()
            .galley(rect.min, Arc::clone(galley), palette::GRAY_2_TEXT);
        drop(cache);
        response
    }

    /// Returns the placeholder of the search field.
    pub(super) fn hint(&self) -> Arc<str> {
        self.lock().hint.clone().unwrap_or_default()
    }

    /// Hands the text of the search field to `edit`, set to `current` first when it differs.
    pub(super) fn edit(&self, current: &str, edit: impl FnOnce(&mut String)) {
        let mut cache = self.lock();
        if cache.text != current {
            current.clone_into(&mut cache.text);
        }
        edit(&mut cache.text);
    }

    /// Returns `note`, laid out from what `make` returns the first time it is asked for.
    pub(super) fn note(
        &self,
        ui: &Ui,
        note: Note,
        make: impl FnOnce() -> (String, Color32),
    ) -> Arc<Galley> {
        let mut cache = self.lock();
        Arc::clone(cache.notes.entry(note).or_insert_with(|| {
            let (text, colour) = make();
            line(
                ui,
                plain(text, FS_XS, Weight::Regular).color(colour),
                f32::INFINITY,
            )
        }))
    }

    fn cell(&self, ui: &Ui, key: (RecordId, u8, bool), text: &str, width: f32) -> Arc<Galley> {
        let mut cache = self.lock();
        Arc::clone(
            cache
                .cells
                .entry(key)
                .or_insert_with(|| line(ui, plain(text, ROW_FONT, row_weight(key.2)), width)),
        )
    }
}

fn cell_width(width: f32) -> f32 {
    2.0f32.mul_add(-CELL_PADDING_X, width / 3.0).max(0.0)
}

fn label(ui: &Ui, texts: &Texts) -> Labelled {
    let name = texts.get(RecordSearchText::Label);
    let galley = line(ui, eyebrow(&name), f32::INFINITY);
    (name, galley)
}

fn heads(ui: &Ui, texts: &Texts, width: f32) -> Vec<Labelled> {
    COLUMNS
        .iter()
        .map(|&column| {
            let name = texts.get(column);
            let text = caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
            let galley = line(ui, text, width);
            (name, galley)
        })
        .collect()
}

/// Shows the matching Records as a table of name, type and record path; returns the Record
/// that was picked.
pub(super) fn show(
    ui: &mut Ui,
    tree: &RecordTree,
    laid: &Laid,
    matches: &[RecordId],
) -> Option<RecordId> {
    let width = ui.available_width();
    let (head, _) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(head, CornerRadius::ZERO, palette::SURFACE_INPUT);
    let rule = Rect::from_min_max(pos2(head.left(), head.bottom() - 2.0), head.max);
    painter.rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
    for (step, (name, galley)) in (0_u8..).zip(&laid.lock().heads) {
        let rect = cell(head, step).with_max_y(head.bottom() - 2.0);
        labelled(ui, ("record-head", step), name, rect);
        paint_left(ui, rect, Arc::clone(galley), palette::GRAY_1);
    }
    let selected = tree
        .selected()
        .and_then(|node| match tree.node(node).kind() {
            RecordNodeKind::Record(id) => Some(id),
            RecordNodeKind::Folder | RecordNodeKind::RecordFolder => None,
        });
    let mut picked = None;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical()
        .id_salt("record-matches")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, matches.len(), |ui, visible| {
            for index in visible {
                let Some((&id, record)) = matches
                    .get(index)
                    .and_then(|id| tree.record(*id).map(|record| (id, record)))
                else {
                    continue;
                };
                let row = Row {
                    id,
                    record,
                    active: selected == Some(id),
                    odd: index % 2 == 1,
                };
                if row.show(ui, laid) {
                    picked = Some(id);
                }
            }
        });
    picked
}

fn cell(row: Rect, step: u8) -> Rect {
    let width = row.width() / 3.0;
    let left = f32::from(step).mul_add(width, row.left());
    Rect::from_min_size(pos2(left, row.top()), vec2(width, row.height()))
        .shrink2(vec2(CELL_PADDING_X, 0.0))
}

fn labelled(ui: &Ui, salt: impl egui::AsIdSalt, name: &str, rect: Rect) {
    let response = ui.interact(rect, ui.scope_id().with(salt), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, name));
}

struct Row<'a> {
    id: RecordId,
    record: &'a RecordInfo,
    active: bool,
    odd: bool,
}

impl Row<'_> {
    fn show(&self, ui: &mut Ui, laid: &Laid) -> bool {
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), ROW_HEIGHT));
        let state = RowState::of(self.active, false, ui.rect_contains_pointer(rect));
        if state == RowState::Rest && self.odd {
            ui.painter().rect_filled(rect, CornerRadius::ZERO, ZEBRA);
        }
        paint_row(ui.painter(), rect, state);
        paint_divider(ui.painter(), rect);
        let row_id = ui.scope_id().with(("record-match", self.id));
        let texts = [
            self.record.name(),
            self.record.type_name(),
            self.record.record_path(),
        ];
        let width = cell_width(rect.width());
        for (step, text) in (0_u8..).zip(texts) {
            let cell_rect = cell(rect, step);
            labelled(ui, ("record-cell", self.id, step), text, cell_rect);
            let galley = laid.cell(ui, (self.id, step, self.active), text, width);
            paint_left(ui, cell_rect, galley, state.colours().1);
        }
        let response = ui.interact(rect, row_id, Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(Role::Button, true, self.active, self.record.name())
        });
        focus_inside(ui.painter(), rect, &response);
        response
            .on_hover_ui(|ui| {
                ui.label(self.record.name());
                ui.label(self.record.type_name());
                ui.label(self.record.record_path());
                ui.label(self.record.guid().to_string());
            })
            .clicked()
    }
}
