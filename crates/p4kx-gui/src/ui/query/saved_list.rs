use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    Align, Color32, Galley, Id, Label, Layout, Rect, Role, ScrollArea, Sense, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};
use p4kx_settings::SavedQueries;

use crate::theme::desktop::{
    CHANNEL_GAP, ROW_HEIGHT, SAVED_LIST_HEIGHT, SAVED_NAME_MAX_WIDTH, SIDEBAR_PADDING_X,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, Weight, font};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_row, section_header};
use crate::widgets::scroll_region;
use crate::widgets::text::{line, plain};
use crate::{QueryText, ScrollText, Texts};

/// What the user did with a saved Query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SavedAction {
    Run(usize),
    Rename(usize),
    Delete(usize),
}

/// The most characters of a Query text a row shows.
const SHOWN_TEXT: usize = 240;

/// The laid-out names, texts and actions of the saved Queries, made again only when they, the
/// language or the scale change.
struct Laid {
    key: u64,
    title: String,
    none: Arc<Galley>,
    rename: (String, Arc<Galley>),
    delete: (String, Arc<Galley>),
    actions_width: f32,
    rows: Vec<Row>,
}

/// One saved Query laid out: its name, elided to one line, its text and the accessible name
/// of the row.
struct Row {
    name: Arc<Galley>,
    text: Arc<Galley>,
    run: String,
}

/// Shows the saved Queries as a collapsible section of rows; a row runs its Query, its actions
/// rename and delete it. Only the rows in view are shown.
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
        ui.add(Label::new(Arc::clone(&laid.none)).selectable(false));
        return None;
    }
    let mut action = None;
    let area = ScrollArea::vertical()
        .id_salt("p4kx-saved-queries-list")
        .max_height(SAVED_LIST_HEIGHT)
        .auto_shrink([false, true]);
    let id = ui.scope_id().with("p4kx-saved-queries-list");
    scroll_region::show(
        ui,
        (id, || texts.get(ScrollText::SavedQueries)),
        ROW_HEIGHT,
        area,
        |area, ui| {
            area.show_rows(ui, ROW_HEIGHT, laid.rows.len(), |ui, range| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for index in range {
                    if let Some(row) = laid.rows.get(index)
                        && let Some(done) = saved_row(ui, index, row, &laid)
                    {
                        action = Some(done);
                    }
                }
            })
        },
    );
    action
}

fn saved_row(ui: &mut Ui, index: usize, row: &Row, laid: &Laid) -> Option<SavedAction> {
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
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, row.run.as_str()));
    paint_row(
        ui.painter(),
        rect,
        RowState::of(false, false, response.hovered()),
    );
    let left = body.left() + SIDEBAR_PADDING_X;
    let y = |galley: &Galley| rect.center().y - galley.size().y / 2.0;
    let painter = ui.painter().with_clip_rect(body);
    painter.galley(
        pos2(left, y(&row.name)),
        Arc::clone(&row.name),
        palette::GRAY_1,
    );
    let text_left = left + row.name.size().x + CHANNEL_GAP;
    painter.galley(
        pos2(text_left, y(&row.text)),
        Arc::clone(&row.text),
        palette::GRAY_2_TEXT,
    );
    focus_inside(ui.painter(), rect, &response);
    paint_divider(ui.painter(), rect);
    if response.clicked() {
        action = Some(SavedAction::Run(index));
    }
    let actions = Rect::from_min_max(pos2(body.right(), rect.top()), rect.max);
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(actions.shrink2(vec2(CHANNEL_GAP, 0.0)))
            .layout(Layout::right_to_left(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(CHANNEL_GAP, 0.0);
            ui.push_id(index, |ui| {
                let (delete, galley) = &laid.delete;
                let shown = (delete.as_str(), Arc::clone(galley));
                let kind = (Kind::QuietDanger, Size::Compact);
                if button::button_laid(ui, kind, shown, true).clicked() {
                    action = Some(SavedAction::Delete(index));
                }
                let (rename, galley) = &laid.rename;
                let shown = (rename.as_str(), Arc::clone(galley));
                if button::button_laid(ui, (Kind::Ghost, Size::Compact), shown, true).clicked() {
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
            let flat: String = query
                .text()
                .split_whitespace()
                .flat_map(|word| [" ", word])
                .skip(1)
                .flat_map(str::chars)
                .take(SHOWN_TEXT)
                .collect();
            Row {
                name: elided(ui, query.name()),
                text: line(ui, plain(flat, FS_XS, Weight::Regular), f32::INFINITY),
                run: texts.format(
                    QueryText::SavedRun,
                    &[("name", query.name().to_owned().into())],
                ),
            }
        })
        .collect();
    let none = texts.get(QueryText::SavedNone);
    let action = |text: QueryText| {
        let label = texts.get(text);
        let galley = button::laid_label(ui, Size::Compact, &label);
        (label, galley)
    };
    let rename = action(QueryText::SavedRename);
    let delete = action(QueryText::SavedDelete);
    let width = |galley: &Galley| button::width_of(Size::Compact, galley);
    let actions_width = CHANNEL_GAP.mul_add(3.0, width(&rename.1) + width(&delete.1));
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

/// Lays `name` out bold on one line, ending in an ellipsis beyond the widest a name gets.
fn elided(ui: &Ui, name: &str) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(
        name.to_owned(),
        TextFormat::simple(font(FS_XS, Weight::Bold), Color32::PLACEHOLDER),
    );
    job.wrap = TextWrapping::truncate_at_width(SAVED_NAME_MAX_WIDTH);
    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
}
