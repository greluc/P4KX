use std::sync::Arc;

use egui::{Align, Galley, Id, Layout, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use p4kx_settings::SavedQueries;

use crate::theme::desktop::{CHANNEL_GAP, ROW_HEIGHT, SAVED_LIST_HEIGHT, SIDEBAR_PADDING_X};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_row, section_header};
use crate::widgets::text::{line, plain};
use crate::{QueryText, Texts};

/// What the user did with a saved Query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SavedAction {
    Run(usize),
    Rename(usize),
    Delete(usize),
}

/// The laid-out names and texts of the saved Queries, made again only when they or the
/// language change.
struct Laid {
    key: u64,
    title: String,
    none: Arc<Galley>,
    rename: String,
    delete: String,
    actions_width: f32,
    rows: Vec<(Arc<Galley>, Arc<Galley>, String)>,
}

/// The most characters of a Query text a row shows.
const SHOWN_TEXT: usize = 240;

/// Shows the saved Queries as a collapsible section of rows; a row runs its Query, its actions
/// rename and delete it.
pub(super) fn show(
    ui: &mut Ui,
    saved: &SavedQueries,
    (texts, key): (&Texts, u64),
) -> Option<SavedAction> {
    let laid = laid(ui, saved, texts, key);
    let open = section_header(ui, Id::unique("p4kx-saved-queries"), &laid.title, true);
    if !open {
        return None;
    }
    if laid.rows.is_empty() {
        ui.add_space(CHANNEL_GAP);
        ui.add(egui::Label::new(Arc::clone(&laid.none)).selectable(false));
        return None;
    }
    let mut action = None;
    ScrollArea::vertical()
        .id_salt("p4kx-saved-queries-list")
        .max_height(SAVED_LIST_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for (index, row) in laid.rows.iter().enumerate() {
                if let Some(done) = saved_row(ui, index, row, &laid) {
                    action = Some(done);
                }
            }
        });
    action
}

fn saved_row(
    ui: &mut Ui,
    index: usize,
    (name, text, run): &(Arc<Galley>, Arc<Galley>, String),
    laid: &Laid,
) -> Option<SavedAction> {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::hover());
    let mut action = None;
    let body = Rect::from_min_max(
        rect.min,
        pos2(rect.right() - laid.actions_width, rect.bottom()),
    );
    let response = ui.interact(
        body,
        ui.scope_id().with(("saved-query", index)),
        Sense::click(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, run.as_str()));
    let state = RowState::of(false, false, response.hovered());
    paint_row(ui.painter(), rect, state);
    let left = body.left() + SIDEBAR_PADDING_X;
    let y = |galley: &Galley| rect.center().y - galley.size().y / 2.0;
    ui.painter()
        .galley(pos2(left, y(name)), Arc::clone(name), palette::GRAY_1);
    let text_left = left + name.size().x + CHANNEL_GAP;
    let clip = Rect::from_min_max(
        pos2(text_left, rect.top()),
        pos2(body.right(), rect.bottom()),
    );
    ui.painter().with_clip_rect(clip).galley(
        pos2(text_left, y(text)),
        Arc::clone(text),
        palette::GRAY_2_TEXT,
    );
    focus_inside(ui.painter(), rect, &response);
    paint_divider(ui.painter(), rect);
    if response.clicked() {
        action = Some(SavedAction::Run(index));
    }
    let actions = Rect::from_min_max(pos2(body.right(), rect.top()), rect.max);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(actions.shrink2(vec2(CHANNEL_GAP, 0.0)))
            .layout(Layout::right_to_left(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(CHANNEL_GAP, 0.0);
            ui.push_id(index, |ui| {
                if button::button(ui, Kind::QuietDanger, Size::Compact, &laid.delete, true)
                    .clicked()
                {
                    action = Some(SavedAction::Delete(index));
                }
                if button::button(ui, Kind::Ghost, Size::Compact, &laid.rename, true).clicked() {
                    action = Some(SavedAction::Rename(index));
                }
            });
        },
    );
    action
}

fn laid(ui: &Ui, saved: &SavedQueries, texts: &Texts, key: u64) -> Arc<Laid> {
    let key = key ^ u64::from(ui.ctx().pixels_per_point().to_bits());
    let id = Id::unique("p4kx-saved-queries-laid");
    if let Some(laid) = ui.data(|data| data.get_temp::<Arc<Laid>>(id))
        && laid.key == key
    {
        return laid;
    }
    let rows = saved
        .iter()
        .map(|query| {
            let name = line(ui, plain(query.name(), FS_XS, Weight::Bold), f32::INFINITY);
            let flat: String = query
                .text()
                .split_whitespace()
                .flat_map(|word| [" ", word])
                .skip(1)
                .flat_map(str::chars)
                .take(SHOWN_TEXT)
                .collect();
            let text = line(ui, plain(flat, FS_XS, Weight::Regular), f32::INFINITY);
            let run = texts.format(
                QueryText::SavedRun,
                &[("name", query.name().to_owned().into())],
            );
            (name, text, run)
        })
        .collect();
    let none = texts.get(QueryText::SavedNone);
    let rename = texts.get(QueryText::SavedRename);
    let delete = texts.get(QueryText::SavedDelete);
    let actions_width = CHANNEL_GAP.mul_add(
        3.0,
        button::width(ui, Size::Compact, &rename) + button::width(ui, Size::Compact, &delete),
    );
    let laid = Arc::new(Laid {
        key,
        title: texts.get(QueryText::SavedTitle),
        none: line(
            ui,
            plain(none, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
            ui.available_width(),
        ),
        rename,
        delete,
        actions_width,
        rows,
    });
    ui.data_mut(|data| data.insert_temp(id, Arc::clone(&laid)));
    laid
}
