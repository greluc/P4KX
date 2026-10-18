use egui::{Rect, Role, ScrollArea, Sense, Stroke, Ui, WidgetInfo, pos2, vec2};

use crate::theme::desktop::{
    CHEVRON, ROW_FONT, ROW_HEIGHT, SCROLLBAR_THIN, TREE_GAP, TREE_INDENT, TREE_PADDING_LEFT,
    TREE_RAIL_OFFSET,
};
use crate::theme::palette;
use crate::theme::tokens::{SPACE_2, Weight};
use crate::widgets::rows::{RowState, chevron, paint_row};
use crate::widgets::text::{line, paint_left, plain};
use crate::{Browser, FolderId, Model, Text, Texts, TreeRow};

enum Action {
    Select(FolderId),
    Toggle(FolderId),
}

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let rows = browser.tree_rows();
    let mut action = None;
    ui.spacing_mut().scroll.bar_width = SCROLLBAR_THIN;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        ROW_HEIGHT,
        rows.len(),
        |ui, visible| {
            for row in rows.get(visible).unwrap_or_default() {
                if let Some(clicked) = show_row(ui, texts, browser, *row) {
                    action = Some(clicked);
                }
            }
        },
    );
    if let (Some(action), Some(browser)) = (action, model.browser_mut()) {
        match action {
            Action::Select(folder) => browser.select_folder(folder),
            Action::Toggle(folder) => browser.toggle_expanded(folder),
        }
    }
}

fn show_row(ui: &mut Ui, texts: &Texts, browser: &Browser, row: TreeRow) -> Option<Action> {
    let index = browser.index();
    let folder = index.folder(row.folder);
    let root_name;
    let name = if row.folder == index.root() {
        root_name = index
            .archive()
            .path()
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        root_name.as_str()
    } else {
        folder.name()
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let active = !browser.shows_search() && browser.selected_folder() == row.folder;
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, active, name));
    let state = RowState::of(active, false, response.hovered());
    paint_row(ui.painter(), rect, state);
    let depth = depth(row.depth);
    for level in 0..u16::try_from(row.depth).unwrap_or(u16::MAX) {
        let x = f32::from(level).mul_add(
            TREE_INDENT,
            rect.left() + TREE_PADDING_LEFT + TREE_RAIL_OFFSET,
        );
        ui.painter()
            .vline(x, rect.y_range(), Stroke::new(1.0, palette::GRAY_3));
    }
    let chevron_left = depth.mul_add(TREE_INDENT, rect.left() + TREE_PADDING_LEFT);
    let chevron_rect = Rect::from_min_size(
        pos2(chevron_left, rect.top()),
        vec2(CHEVRON + TREE_GAP, ROW_HEIGHT),
    );
    let expanded = browser.is_expanded(row.folder);
    let mut action = (!folder.children().is_empty()
        && toggle(ui, texts, (row.folder, chevron_rect), expanded, name))
    .then_some(Action::Toggle(row.folder));
    let (_, colour) = state.colours();
    let weight = if active {
        Weight::Bold
    } else {
        Weight::Regular
    };
    let text_rect = rect.with_min_x(chevron_rect.right());
    let galley = line(
        ui,
        plain(name, ROW_FONT, weight),
        (text_rect.width() - SPACE_2).max(0.0),
    );
    paint_left(ui, text_rect, galley, colour);
    if response
        .on_hover_ui(|ui| {
            ui.label(name);
        })
        .clicked()
        && action.is_none()
    {
        action = Some(Action::Select(row.folder));
    }
    action
}

fn toggle(
    ui: &Ui,
    texts: &Texts,
    (folder, rect): (FolderId, Rect),
    expanded: bool,
    name: &str,
) -> bool {
    let text = if expanded {
        Text::CollapseFolder
    } else {
        Text::ExpandFolder
    };
    let response = ui.interact(rect, ui.scope_id().with(("toggle", folder)), Sense::click());
    response.widget_info(|| {
        let label = texts.format(text, &[("name", name.to_owned().into())]);
        WidgetInfo::labeled(Role::Button, true, &label)
    });
    let centre = pos2(rect.left() + CHEVRON / 2.0, rect.center().y);
    chevron(ui.painter(), centre, CHEVRON, expanded, palette::PRIMARY);
    response.clicked()
}

fn depth(depth: usize) -> f32 {
    f32::from(u16::try_from(depth).unwrap_or(u16::MAX))
}
