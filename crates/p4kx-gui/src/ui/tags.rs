use egui::{
    Align, Color32, CornerRadius, Label, Layout, Margin, Rect, Role, ScrollArea, Sense, Ui,
    WidgetInfo, pos2, vec2,
};
use fluent_bundle::FluentValue;

use super::search::{eyebrow, field};
use super::tree::{rails, toggle};
use super::{empty, list_footer};
use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, SCROLLBAR_THIN, SPINNER, SPINNER_INLINE, TABLE_HEAD_SIZE,
    TABLE_HEAD_TRACKING, TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::theme::tags_metrics::{TAGS_GUID_WIDTH, TAGS_LEGACY_WIDTH};
use crate::theme::tokens::{FS_SM, SPACE_2, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_row, row_weight};
use crate::widgets::text::{caps, hairline_below, line, paint_left, plain};
use crate::widgets::tooltip;
use crate::{Model, TagRow, TagsStatus, TagsText, TagsView, Texts, format};

/// What the user did in the tree: the position in the pre-order of the tag it is about.
#[derive(Clone, Copy)]
enum Action {
    Select(usize),
    Toggle(usize),
}

/// Shows the Tags tab: the search and copy actions, then the tag tree with the number of tags
/// below it, or the state of reading the tags.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(TagsText::EmptyTitle);
        let hint = texts.get(TagsText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    if !model.tags().is_started() {
        let ctx = ui.ctx().clone();
        model.load_tags(move || ctx.request_repaint());
    }
    if matches!(model.tags().status(), TagsStatus::Ready(_)) {
        let toolbar = frames::panel()
            .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
            .show(ui, |ui| toolbar(ui, model));
        hairline_below(ui, toolbar.response.rect);
    }
    let action = frames::panel()
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            body(ui, model)
        })
        .inner;
    match action {
        Some(Action::Select(position)) => model.tags_mut().select(position),
        Some(Action::Toggle(position)) => model.tags_mut().toggle(position),
        None => {}
    }
}

fn toolbar(ui: &mut Ui, model: &mut Model) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let texts = model.texts();
    let label = eyebrow(&texts.get(TagsText::Label));
    let hint = texts.get(TagsText::Hint);
    let copy_path = texts.get(TagsText::CopyPath);
    let copy_guid = texts.get(TagsText::CopyGuid);
    let tab = model.tags();
    let mut text = tab.query().to_owned();
    let path = tab.selected_path().map(str::to_owned);
    let guid = tab.selected_guid().map(|guid| guid.to_string());
    let mut changed = false;
    let mut copy = None;
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button::button(ui, Kind::Ghost, Size::Normal, &copy_guid, guid.is_some()).clicked() {
                copy.clone_from(&guid);
            }
            if button::button(ui, Kind::Ghost, Size::Normal, &copy_path, path.is_some()).clicked() {
                copy.clone_from(&path);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                changed = field(ui, &mut text, &hint, label.id);
            });
        });
    });
    if let Some(copy) = copy {
        ui.ctx().copy_text(copy);
    }
    if changed {
        model.tags_mut().set_query(&text);
    }
}

fn body(ui: &mut Ui, model: &Model) -> Option<Action> {
    let texts = model.texts();
    match model.tags().status() {
        TagsStatus::Idle | TagsStatus::Loading => loading(ui, texts),
        TagsStatus::NoTags => message(ui, texts.get(TagsText::NoTags), palette::GRAY_2_TEXT),
        TagsStatus::Failed(Some(reason)) => {
            let text = texts.format(TagsText::Failed, &[("reason", reason.to_owned().into())]);
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        TagsStatus::Failed(None) => {
            let text = texts.get(TagsText::Stopped);
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        TagsStatus::ArchiveChanged => {
            let text = texts.get(TagsText::ArchiveChanged);
            frames::warning_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        TagsStatus::Ready(view) => return ready(ui, texts, view),
    }
    None
}

fn ready(ui: &mut Ui, texts: &Texts, view: &TagsView) -> Option<Action> {
    if view.irregularities() > 0 {
        let number = format::number(
            u64::try_from(view.irregularities()).unwrap_or(u64::MAX),
            texts.language(),
        );
        let text = texts.format(TagsText::Irregular, &[("number", number.into())]);
        frames::warning_alert(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(Label::new(text).selectable(true));
        });
    }
    let total = view.tree().len();
    let count = count(
        texts,
        if view.is_filtered() {
            view.shown_len()
        } else {
            total
        },
        total,
    );
    list_footer::show(ui, "tags-footer", |ui| {
        ui.add(
            Label::new(plain(count, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
    });
    if view.shown_len() == 0 {
        message(ui, texts.get(TagsText::NoMatch), palette::GRAY_2_TEXT);
        return None;
    }
    head(ui, texts);
    rows(ui, texts, view)
}

fn count(texts: &Texts, shown: usize, total: usize) -> String {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let count = FluentValue::from(u64::try_from(total).unwrap_or(u64::MAX));
    if shown == total {
        texts.format(
            TagsText::Count,
            &[("count", count), ("number", number(total).into())],
        )
    } else {
        texts.format(
            TagsText::CountShown,
            &[
                ("count", count),
                ("number", number(shown).into()),
                ("total", number(total).into()),
            ],
        )
    }
}

/// Returns the cells of a row from the right: the GUID, the legacy ID, and where the tag ends.
fn columns(rect: Rect) -> (Rect, Rect, f32) {
    let legacy = Rect::from_min_max(pos2(rect.right() - TAGS_LEGACY_WIDTH, rect.top()), rect.max);
    let guid = Rect::from_min_max(
        pos2(legacy.left() - TAGS_GUID_WIDTH, rect.top()),
        pos2(legacy.left(), rect.bottom()),
    );
    (guid, legacy, guid.left() - SPACE_2)
}

/// Paints the head of the columns: the table head of the design system with its orange rule.
fn head(ui: &mut Ui, texts: &Texts) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
    let rule = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
    ui.painter()
        .rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
    let (guid, legacy, tag_right) = columns(rect);
    let cells = [
        (
            Rect::from_min_max(rect.min, pos2(tag_right, rect.bottom())),
            TagsText::ColumnTag,
        ),
        (guid, TagsText::ColumnGuid),
        (legacy, TagsText::ColumnLegacy),
    ];
    for (cell, text) in cells {
        let name = texts.get(text);
        let inner = cell.shrink2(vec2(CELL_PADDING_X, 0.0));
        let galley = line(
            ui,
            caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING),
            inner.width().max(0.0),
        );
        paint_left(ui, inner, galley, palette::GRAY_1);
        let id = ui.scope_id().with(("tags-head", text.id()));
        ui.interact(cell, id, Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::ColumnHeader, true, &name));
    }
}

fn rows(ui: &mut Ui, texts: &Texts, view: &TagsView) -> Option<Action> {
    let mut action = None;
    ui.spacing_mut().scroll.bar_width = SCROLLBAR_THIN;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical()
        .id_salt("tags-rows")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, view.shown_len(), |ui, visible| {
            for index in visible {
                if let Some(row) = view.row(index)
                    && let Some(clicked) = show_row(ui, texts, &row, index)
                {
                    action = Some(clicked);
                }
            }
        });
    action
}

fn show_row(ui: &mut Ui, texts: &Texts, row: &TagRow<'_>, index: usize) -> Option<Action> {
    let tag = row.tag();
    let position = tag.position();
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    response
        .widget_info(|| WidgetInfo::selected(Role::Button, true, row.is_selected(), tag.path()));
    let state = RowState::of_tree(row.is_selected(), response.hovered());
    paint_row(ui.painter(), rect, state);
    paint_divider(ui.painter(), rect);
    let chevron = rails(ui, rect, tag.depth());
    let mut action = None;
    if row.is_expandable() {
        let id = ui.scope_id().with(("tags-toggle", position, index));
        if toggle(ui, texts, (id, chevron), row.is_expanded(), tag.name()) {
            action = Some(Action::Toggle(position));
        }
    }
    paint_cells(ui, (rect, chevron.right()), row, state);
    focus_inside(ui.painter(), rect, &response);
    let response = tooltip::hover(response, |ui| {
        tooltip::line(ui, tag.path());
    });
    if response.double_clicked() && row.is_expandable() {
        return Some(Action::Toggle(position));
    }
    if response.clicked() && action.is_none() {
        action = Some(Action::Select(position));
    }
    action
}

/// Paints the name, the GUID and the legacy ID of a row at `rect`, the name from `name_left`.
fn paint_cells(ui: &Ui, (rect, name_left): (Rect, f32), row: &TagRow<'_>, state: RowState) {
    let tag = row.tag();
    let (guid, legacy, tag_right) = columns(rect);
    let (_, colour) = state.colours();
    let muted = if row.is_selected() {
        colour
    } else {
        palette::GRAY_2_TEXT
    };
    let tag_rect = Rect::from_min_max(pos2(name_left, rect.top()), pos2(tag_right, rect.bottom()));
    let name = line(
        ui,
        plain(tag.name(), ROW_FONT, row_weight(row.is_selected())),
        tag_rect.width().max(0.0),
    );
    paint_left(ui, tag_rect, name, colour);
    let cells = [
        (guid, tag.guid().to_string()),
        (
            legacy,
            tag.legacy_id()
                .map_or_else(String::new, |id| id.to_string()),
        ),
    ];
    for (cell, text) in cells {
        let inner = cell.shrink2(vec2(CELL_PADDING_X, 0.0));
        let galley = line(
            ui,
            plain(text, ROW_FONT, Weight::Regular),
            inner.width().max(0.0),
        );
        paint_left(ui, inner, galley, muted);
    }
}

fn loading(ui: &mut Ui, texts: &Texts) {
    ui.add_space(SPINNER);
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.add(
            Label::new(
                plain(texts.get(TagsText::Loading), FS_SM, Weight::Regular).color(palette::GRAY_1),
            )
            .selectable(false),
        );
    });
}

fn message(ui: &mut Ui, text: String, colour: Color32) {
    frames::preview_body().show(ui, |ui| {
        ui.add(Label::new(
            plain(text, FS_SM, Weight::Regular).color(colour),
        ));
    });
}
