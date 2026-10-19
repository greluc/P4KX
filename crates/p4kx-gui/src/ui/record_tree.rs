use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use egui::{Color32, Galley, Id, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, vec2};

use super::dataforge_summary::message;
use super::tree::{rails, toggle};
use crate::theme::desktop::{CHANNEL_GAP, ROW_FONT, ROW_HEIGHT, SPINNER_INLINE};
use crate::theme::palette;
use crate::theme::tokens::{SPACE_2, Weight};
use crate::widgets::indicator::spinner;
use crate::widgets::rows::{RowState, focus_inside, paint_row, row_weight};
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::tooltip::hover;
use crate::{
    DataForgeText, Language, RecordNodeId, RecordNodeKind, RecordRow, RecordTree, RecordTreeStatus,
    Texts,
};

const MAX_CACHED: usize = 4096;

/// What a click on a Record tree row asks for.
pub(super) enum RecordAction {
    Select(RecordNodeId),
    Toggle(RecordNodeId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Key {
    generation: u64,
    language: Language,
    status: u8,
    width: f32,
    pixels_per_point: f32,
}

#[derive(Default)]
struct Cache {
    key: Option<Key>,
    names: HashMap<(RecordNodeId, bool), Arc<Galley>>,
    note: Option<(Arc<Galley>, Color32)>,
    root: Option<Arc<str>>,
}

/// The laid-out names of the Record tree rows, kept across frames until the loaded Records, the
/// language, the status or the width change.
#[derive(Clone)]
pub(super) struct Labels(Arc<Mutex<Cache>>);

impl Labels {
    pub(super) fn get(ui: &Ui, tree: &RecordTree, texts: &Texts) -> Self {
        let id = Id::unique("p4kx-record-labels");
        let labels = ui.data_mut(|data| {
            data.get_temp_mut_or_insert_with(id, || Self(Arc::new(Mutex::new(Cache::default()))))
                .clone()
        });
        let key = Key {
            generation: tree.generation(),
            language: texts.language(),
            status: status_code(tree.status()),
            width: ui.available_width(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        let mut cache = labels.0.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.key != Some(key) || cache.names.len() > MAX_CACHED {
            *cache = Cache {
                key: Some(key),
                ..Cache::default()
            };
        }
        drop(cache);
        labels
    }

    fn name(&self, ui: &Ui, node: (RecordNodeId, bool), text: &str, width: f32) -> Arc<Galley> {
        let mut cache = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(
            cache
                .names
                .entry(node)
                .or_insert_with(|| line(ui, plain(text, ROW_FONT, row_weight(node.1)), width)),
        )
    }

    fn root_name(&self, texts: &Texts) -> Arc<str> {
        let mut cache = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(
            cache
                .root
                .get_or_insert_with(|| texts.get(DataForgeText::Node).into()),
        )
    }

    fn note(
        &self,
        ui: &Ui,
        (texts, tree): (&Texts, &RecordTree),
        width: f32,
    ) -> Option<(Arc<Galley>, Color32)> {
        let mut cache = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.note.is_none() {
            let (text, colour) = note_text(texts, tree);
            let galley = line(ui, plain(text, ROW_FONT, Weight::Regular), width);
            cache.note = Some((galley, colour));
        }
        cache.note.clone()
    }
}

/// Returns a number for each kind of `status`, for cache keys.
pub(super) const fn status_code(status: RecordTreeStatus<'_>) -> u8 {
    match status {
        RecordTreeStatus::NotLoaded => 0,
        RecordTreeStatus::Loading => 1,
        RecordTreeStatus::Failed(_) => 2,
        RecordTreeStatus::ArchiveChanged => 3,
        RecordTreeStatus::Ready(_) => 4,
    }
}

fn note_text(texts: &Texts, tree: &RecordTree) -> (String, Color32) {
    match tree.status() {
        RecordTreeStatus::Loading => (texts.get(DataForgeText::NodeLoading), palette::GRAY_2_TEXT),
        RecordTreeStatus::NotLoaded | RecordTreeStatus::Ready(_) => {
            let path = tree.entry_path();
            let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
            (name.to_owned(), palette::GRAY_2_TEXT)
        }
        status => message(texts, tree.entry_path(), status)
            .unwrap_or_else(|| (String::new(), palette::GRAY_2_TEXT)),
    }
}

/// Shows one row of the Record tree; returns what a click on it asks for.
pub(super) fn show_row(
    ui: &mut Ui,
    (texts, tree, labels): (&Texts, &RecordTree, &Labels),
    row: RecordRow,
) -> Option<RecordAction> {
    let node = tree.node(row.node);
    let root = row.node == tree.root();
    let root_name;
    let name = if root {
        root_name = labels.root_name(texts);
        &*root_name
    } else {
        node.name()
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let active = tree.selected() == Some(row.node);
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, active, name));
    let state = RowState::of_list(active, response.hovered());
    paint_row(ui.painter(), rect, state);
    let chevron_rect = rails(ui, rect, row.depth);
    let toggle_id = ui.scope_id().with(("record-toggle", row.node));
    let expanded = tree.is_expanded(row.node);
    let mut action = (tree.is_expandable(row.node)
        && toggle(ui, texts, (toggle_id, chevron_rect), expanded, name))
    .then_some(RecordAction::Toggle(row.node));
    let text_rect = rect.with_min_x(chevron_rect.right());
    let width = (text_rect.width() - SPACE_2).max(0.0);
    let galley = labels.name(ui, (row.node, active), name, width);
    let name_right = text_rect.left() + galley.size().x;
    paint_left(ui, text_rect, galley, state.colours().1);
    if root {
        let note_rect = Rect::from_min_max(
            rect.min + vec2(name_right - rect.left() + CHANNEL_GAP, 0.0),
            rect.max - vec2(SPACE_2, 0.0),
        );
        root_note(ui, (texts, tree, labels), note_rect);
    }
    focus_inside(ui.painter(), rect, &response);
    let clicked = hover(response, |ui| tooltip(ui, texts, tree, row.node)).clicked();
    if clicked && action.is_none() {
        action = Some(RecordAction::Select(row.node));
    }
    action
}

fn root_note(ui: &mut Ui, (texts, tree, labels): (&Texts, &RecordTree, &Labels), rect: Rect) {
    let loading = matches!(tree.status(), RecordTreeStatus::Loading);
    let text_rect = if loading {
        rect.with_min_x(rect.left() + SPINNER_INLINE + SPACE_2)
    } else {
        rect
    };
    let Some((galley, colour)) = labels.note(ui, (texts, tree), text_rect.width().max(0.0)) else {
        return;
    };
    if loading {
        let spot = Rect::from_min_size(rect.min, vec2(SPINNER_INLINE, rect.height()));
        ui.scope_builder(
            UiBuilder::new().max_rect(spot.shrink2(vec2(0.0, (ROW_HEIGHT - SPINNER_INLINE) / 2.0))),
            |ui| spinner(ui, SPINNER_INLINE),
        );
    }
    if !matches!(
        tree.status(),
        RecordTreeStatus::NotLoaded | RecordTreeStatus::Ready(_)
    ) {
        let response = ui.interact(
            text_rect,
            ui.scope_id().with("dataforge-note"),
            Sense::hover(),
        );
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, galley.text()));
    }
    paint_left(ui, text_rect, galley, colour);
}

fn tooltip(ui: &mut Ui, texts: &Texts, tree: &RecordTree, node: RecordNodeId) {
    if node == tree.root() {
        ui.label(tree.entry_path());
        if let Some((text, colour)) = message(texts, tree.entry_path(), tree.status()) {
            ui.colored_label(colour, text);
        }
        return;
    }
    let shown = tree.node(node);
    match shown.kind() {
        RecordNodeKind::Record(id) => {
            if let Some(record) = tree.record(id) {
                ui.label(record.name());
                ui.label(record.type_name());
                ui.label(record.guid().to_string());
            }
        }
        RecordNodeKind::Folder | RecordNodeKind::RecordFolder => {
            ui.label(shown.name());
        }
    }
}
