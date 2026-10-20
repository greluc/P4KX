use std::sync::Arc;

use egui::{
    Align, CornerRadius, Galley, Label, Layout, Rect, Role, Sense, Ui, WidgetInfo, pos2, vec2,
};
use egui_extras::{Column, TableBuilder, TableRow};
use p4kx_extraction::{Compression, Entry};

use super::table_name;
use crate::theme::desktop::{
    CELL_PADDING_X, CHECK_COLUMN, CHEVRON, COMPRESSION_COLUMN, ENCRYPTION_COLUMN, ROW_FONT,
    ROW_HEIGHT, SIZE_COLUMN, SORT_GAP, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::Weight;
use crate::widgets::rows::{RowState, focus_inside, row_weight, sort_chevron};
use crate::widgets::table_row::RowSlots;
use crate::widgets::text::{caps, line, plain};
use crate::widgets::toggle::{Check, checkbox_cell};
use crate::widgets::tooltip;
use crate::{Browser, Model, SortColumn, SortDirection, Text, Texts, format};

const COLUMNS: [(SortColumn, Text); 4] = [
    (SortColumn::Name, Text::ColumnName),
    (SortColumn::Compression, Text::ColumnCompression),
    (SortColumn::Encryption, Text::ColumnEncryption),
    (SortColumn::Size, Text::ColumnSize),
];

enum Action {
    Sort(SortColumn),
    CheckShown(bool),
    Check(u32, bool),
    Preview(u32),
    Toggle(u32),
}

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let hovered_id = ui.scope_id().with("entries-hovered-row");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let mut hovered_now = None;
    let mut action = None;
    let left = ui.max_rect().left();
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    TableBuilder::new(ui)
        .id_salt("entries")
        .striped(false)
        .resizable(true)
        .auto_shrink(false)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::exact(CHECK_COLUMN))
        .column(Column::remainder().at_least(160.0).clip(true))
        .column(
            Column::initial(COMPRESSION_COLUMN)
                .at_least(60.0)
                .clip(true),
        )
        .column(Column::initial(ENCRYPTION_COLUMN).at_least(60.0).clip(true))
        .column(Column::initial(SIZE_COLUMN).at_least(80.0))
        .header(ROW_HEIGHT, |mut header| {
            header.col(|ui| {
                head_fill(ui, left);
                if checkbox_cell(ui, select_all(browser), &texts.get(Text::SelectAll)) {
                    action = Some(Action::CheckShown(!browser.all_shown_checked()));
                }
            });
            for (column, text) in COLUMNS {
                header.col(|ui| {
                    head_fill(ui, ui.max_rect().left());
                    let sort = browser.sort();
                    let sorted = (sort.column == column).then_some(sort.direction);
                    if sort_header(ui, sorted, &texts.get(text)) {
                        action = Some(Action::Sort(column));
                    }
                });
            }
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, browser.rows().len(), |row| {
                let index = row.index();
                let (clicked, is_hovered) = show_row(row, texts, browser, hovered == Some(index));
                if clicked.is_some() {
                    action = clicked;
                }
                if is_hovered {
                    hovered_now = Some(index);
                }
            });
        });
    remember_hovered(ui, hovered_id, hovered_now);
    if let Some(action) = action {
        apply(ui, model, &action);
    }
}

fn remember_hovered(ui: &Ui, id: egui::Id, row: Option<usize>) {
    ui.data_mut(|data| {
        if let Some(index) = row {
            data.insert_temp(id, index);
        } else {
            data.remove_temp::<usize>(id);
        }
    });
}

fn apply(ui: &Ui, model: &mut Model, action: &Action) {
    let ctx = ui.ctx().clone();
    if let Action::Preview(id) = action {
        super::preview::select(model, *id);
    }
    if let Some(browser) = model.browser_mut() {
        match action {
            Action::Sort(column) => browser.sort_by(*column),
            Action::CheckShown(checked) => browser.set_all_shown_checked(*checked),
            Action::Check(id, checked) => browser.set_checked(*id, *checked),
            Action::Toggle(id) => browser.toggle_texture(*id),
            Action::Preview(id) => browser.select_entry(*id, move || ctx.request_repaint()),
        }
    }
}

const fn select_all(browser: &Browser) -> Check {
    if browser.all_shown_checked() {
        Check::On
    } else if browser.checked_count() > browser.hidden_checked_count() {
        Check::Mixed
    } else {
        Check::Off
    }
}

/// Paints the fill and the orange rule of a head cell from `left` to the cell's right edge.
pub(super) fn head_fill(ui: &Ui, left: f32) {
    let rect = ui.max_rect().with_min_x(left);
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
    let rule = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
    ui.painter()
        .rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
}

fn show_row(
    mut row: TableRow<'_, '_>,
    texts: &Texts,
    browser: &Browser,
    hovered: bool,
) -> (Option<Action>, bool) {
    row.set_hovered(false);
    let Some(&entry_row) = browser.rows().get(row.index()) else {
        return (None, false);
    };
    let id = entry_row.id;
    let Some(entry) = browser.index().entry(id) else {
        return (None, false);
    };
    let name = browser.index().name(id);
    let size = browser
        .index()
        .texture_size(id)
        .unwrap_or_else(|| entry.uncompressed_size());
    let checked = browser.is_checked(id);
    let active = browser.selected_entry() == Some(id);
    let state = RowState::of(active, checked, hovered);
    let (_, colour) = state.colours();
    let weight = row_weight(active);
    let mut action = None;
    let index = row.index();
    let mut slots = None;
    row.col(|ui| {
        slots = Some(RowSlots::reserve(ui));
        let label = texts.format(Text::SelectEntry, &[("name", name.to_owned().into())]);
        let check = if checked { Check::On } else { Check::Off };
        if checkbox_cell(ui, check, &label) {
            action = Some(Action::Check(id, !checked));
        }
    });
    row.col(|ui| {
        if table_name::show(ui, texts, browser, entry_row, (colour, weight)) {
            action = Some(Action::Toggle(id));
        }
    });
    data_cells(&mut row, texts, entry, size, (colour, weight));
    let response = row.response();
    if let Some(slots) = slots {
        slots.paint(&response, state, index % 2 == 1);
    }
    let is_hovered = response.hovered();
    focus_inside(
        &response.ctx.layer_painter(response.layer_id),
        response.rect,
        &response,
    );
    if tooltip::hover(response, |ui| {
        tooltip::line(ui, entry.path());
    })
    .clicked()
        && action.is_none()
    {
        action = Some(Action::Preview(id));
    }
    (action, is_hovered)
}

fn data_cells(
    row: &mut TableRow<'_, '_>,
    texts: &Texts,
    entry: &Entry,
    size: u64,
    (colour, weight): (egui::Color32, Weight),
) {
    row.col(|ui| {
        cell_text(ui, &compression(texts, entry), colour, weight);
    });
    row.col(|ui| {
        cell_text(ui, &encryption(texts, entry), colour, weight);
    });
    row.col(|ui| {
        size_cell(ui, format::size(size, texts.language()), colour, weight);
    });
}

fn size_cell(ui: &mut Ui, size: String, colour: egui::Color32, weight: Weight) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add_space(CELL_PADDING_X);
        ui.add(Label::new(plain(size, ROW_FONT, weight).color(colour)).selectable(false));
    });
}

pub(super) fn cell_text(ui: &mut Ui, text: &str, colour: egui::Color32, weight: Weight) {
    ui.add_space(CELL_PADDING_X);
    ui.add(
        Label::new(plain(text, ROW_FONT, weight).color(colour))
            .truncate()
            .selectable(false),
    );
}

/// Shows the sortable head cell `label`, `sorted` in that direction when it is the sort
/// column; returns whether it was clicked.
pub(super) fn sort_header(ui: &Ui, sorted: Option<SortDirection>, label: &str) -> bool {
    let text = caps(label, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
    let room = 2.0f32.mul_add(-CELL_PADDING_X, ui.max_rect().width()) - CHEVRON - SORT_GAP;
    let galley = line(ui, text, room);
    sort_head(ui, sorted, label, &galley)
}

/// Shows a sortable head cell named `name` with its laid-out `galley`, clipped before the
/// chevron, `sorted` in that direction when it is the sort column; returns whether it was
/// clicked.
pub(super) fn sort_head(
    ui: &Ui,
    sorted: Option<SortDirection>,
    name: &str,
    galley: &Arc<Galley>,
) -> bool {
    let rect = ui.max_rect();
    let response = ui.interact(rect, ui.scope_id().with(name), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    let above_rule = rect.with_max_y(rect.bottom() - 2.0);
    if response.hovered() {
        ui.painter()
            .rect_filled(above_rule, CornerRadius::ZERO, palette::GRAY_3);
    }
    let inner = rect.shrink2(vec2(CELL_PADDING_X, 0.0));
    let room = (inner.width() - CHEVRON - SORT_GAP).max(0.0);
    let colour = if sorted.is_some() {
        palette::WHITE
    } else {
        palette::GRAY_1
    };
    let text = Rect::from_min_size(inner.min, vec2(room, inner.height()));
    ui.painter()
        .with_clip_rect(text.intersect(ui.clip_rect()))
        .galley(
            pos2(text.left(), text.center().y - galley.size().y / 2.0),
            Arc::clone(galley),
            colour,
        );
    let centre = pos2(
        inner.left() + galley.size().x.min(room) + SORT_GAP + CHEVRON / 2.0,
        inner.center().y,
    );
    if let Some(direction) = sorted {
        let up = direction == SortDirection::Ascending;
        sort_chevron(ui.painter(), centre, CHEVRON, up, palette::PRIMARY);
    } else if response.hovered() {
        sort_chevron(ui.painter(), centre, CHEVRON, true, palette::GRAY_2_TEXT);
    }
    focus_inside(ui.painter(), above_rule, &response);
    response.clicked()
}
fn encryption(texts: &Texts, entry: &Entry) -> String {
    texts.get(if entry.is_encrypted() {
        Text::EncryptedYes
    } else {
        Text::EncryptedNo
    })
}

fn compression(texts: &Texts, entry: &Entry) -> String {
    match entry.compression() {
        Compression::Store => texts.get(Text::CompressionStore),
        Compression::Deflate => texts.get(Text::CompressionDeflate),
        Compression::Zstd => texts.get(Text::CompressionZstd),
        Compression::Other(method) => texts.format(
            Text::CompressionOther,
            &[("method", method.to_string().into())],
        ),
    }
}
