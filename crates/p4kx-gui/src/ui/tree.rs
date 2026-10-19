use egui::{Id, Rect, RichText, Role, ScrollArea, Sense, Stroke, Ui, WidgetInfo, pos2, vec2};

use super::container_note::{self, Notes};
use super::record_tree::{self, Labels, RecordAction};
use crate::theme::desktop::{
    CHANNEL_GAP, CHEVRON, FOCUS_OFFSET, ROW_FONT, ROW_HEIGHT, SCROLLBAR_THIN, TREE_GAP,
    TREE_INDENT, TREE_PADDING_LEFT, TREE_RAIL_OFFSET,
};
use crate::theme::palette;
use crate::theme::tokens::SPACE_2;
use crate::widgets::focus::focus_ring_square;
use crate::widgets::rows::{RowState, chevron, focus_inside, paint_row, row_weight};
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::tooltip;
use crate::{Browser, Model, PreviewTab, RecordTree, Text, Texts, TreeNode, TreeRow};

enum Action {
    Select(TreeNode),
    Toggle(TreeNode),
    Record(RecordAction),
}

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let scroll = model.browser_mut().and_then(Browser::take_tree_scroll);
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let rows = browser.tree_rows();
    let records = browser.records();
    let record_rows = records.map_or(&[][..], RecordTree::rows);
    let labels = records.map(|tree| Labels::get(ui, tree, texts));
    let notes = Notes::get(ui, texts);
    let mut action = None;
    ui.spacing_mut().scroll.bar_width = SCROLLBAR_THIN;
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut area = ScrollArea::vertical().auto_shrink(false);
    if let Some(row) = scroll {
        let top = u32::try_from(row).map_or(f64::MAX, f64::from) * f64::from(ROW_HEIGHT);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a scroll offset needs no more than f32 precision"
        )]
        let top = top as f32;
        let above = (ui.available_height() - ROW_HEIGHT).max(0.0) / 2.0;
        area = area.vertical_scroll_offset((top - above).max(0.0));
    }
    area.show_rows(
        ui,
        ROW_HEIGHT,
        rows.len() + record_rows.len(),
        |ui, visible| {
            for index in visible {
                let clicked = if let Some(row) = rows.get(index) {
                    show_row(ui, (texts, &notes), browser, *row)
                } else if let (Some(tree), Some(labels), Some(row)) = (
                    records,
                    labels.as_ref(),
                    record_rows.get(index - rows.len()),
                ) {
                    record_tree::show_row(ui, (texts, tree, labels), *row).map(Action::Record)
                } else {
                    None
                };
                if clicked.is_some() {
                    action = clicked;
                }
            }
        },
    );
    if let Some(action) = action {
        apply(ui, model, &action);
    }
}

fn apply(ui: &Ui, model: &mut Model, action: &Action) {
    let ctx = ui.ctx().clone();
    let wake = move || ctx.request_repaint();
    let Some(browser) = model.browser_mut() else {
        return;
    };
    match *action {
        Action::Select(node) => browser.select_node(node, wake),
        Action::Toggle(node) => browser.toggle_node(node, wake),
        Action::Record(RecordAction::Select(node)) => browser.select_record_node(node, wake),
        Action::Record(RecordAction::Toggle(node)) => browser.toggle_record_node(node, wake),
    }
    if matches!(action, Action::Record(RecordAction::Select(_))) {
        let record = browser.selected_record().is_some();
        model.set_preview_tab(if record {
            PreviewTab::Record
        } else {
            PreviewTab::DataForge
        });
    }
}

fn show_row(
    ui: &mut Ui,
    (texts, notes): (&Texts, &Notes),
    browser: &Browser,
    row: TreeRow,
) -> Option<Action> {
    let node = row.node;
    let root_name;
    let name = if node == TreeNode::from(browser.archive_index().root()) {
        root_name = archive_name(browser);
        root_name.as_str()
    } else {
        browser.node_name(node)
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let active = !browser.shows_search()
        && browser.selected_node() == node
        && browser.records().and_then(RecordTree::selected).is_none();
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, active, name));
    let state = RowState::of_list(active, response.hovered());
    paint_row(ui.painter(), rect, state);
    let chevron_rect = rails(ui, rect, row.depth);
    let expanded = browser.is_expanded(node);
    let toggle_id = ui.scope_id().with(("toggle", node));
    let mut action = (browser.is_expandable(node)
        && toggle(ui, texts, (toggle_id, chevron_rect), expanded, name))
    .then_some(Action::Toggle(node));
    let (_, colour) = state.colours();
    let weight = row_weight(active);

    let text_rect = rect.with_min_x(chevron_rect.right());
    let galley = line(
        ui,
        plain(name, ROW_FONT, weight),
        (text_rect.width() - SPACE_2).max(0.0),
    );
    let name_right = text_rect.left() + galley.size().x;
    paint_left(ui, text_rect, galley, colour);
    let container = node
        .as_container()
        .map(|id| (id, browser.container_status(id)));
    if let Some(container) = container {
        let note_rect = Rect::from_min_max(
            pos2(name_right + CHANNEL_GAP, rect.top()),
            rect.max - vec2(SPACE_2, 0.0),
        );
        container_note::show(ui, (texts, notes), container, note_rect);
    }
    focus_inside(ui.painter(), rect, &response);
    if tooltip::hover(response, |ui| {
        tooltip::line(ui, name);
        if let Some((_, status)) = container
            && let Some((text, colour)) =
                container_note::detail(texts, name, status, browser.container_budget())
        {
            tooltip::line(ui, RichText::new(text).color(colour));
        }
    })
    .clicked()
        && action.is_none()
    {
        action = Some(Action::Select(node));
    }
    action
}

fn archive_name(browser: &Browser) -> String {
    browser
        .archive_index()
        .archive()
        .path()
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// Paints the depth rails of a tree row at `rect` and returns the cell of its chevron.
pub(super) fn rails(ui: &Ui, rect: Rect, depth: usize) -> Rect {
    let levels = u16::try_from(depth).unwrap_or(u16::MAX);
    for level in 0..levels {
        let x = f32::from(level).mul_add(
            TREE_INDENT,
            rect.left() + TREE_PADDING_LEFT + TREE_RAIL_OFFSET,
        );
        ui.painter()
            .vline(x, rect.y_range(), Stroke::new(1.0, palette::GRAY_3));
    }
    let chevron_left = f32::from(levels).mul_add(TREE_INDENT, rect.left() + TREE_PADDING_LEFT);
    Rect::from_min_size(
        pos2(chevron_left, rect.top()),
        vec2(CHEVRON + TREE_GAP, ROW_HEIGHT),
    )
}

/// Shows the chevron that expands or collapses the node called `name`; returns whether it was
/// clicked.
pub(super) fn toggle(
    ui: &Ui,
    texts: &Texts,
    (id, rect): (Id, Rect),
    expanded: bool,
    name: &str,
) -> bool {
    let text = if expanded {
        Text::CollapseFolder
    } else {
        Text::ExpandFolder
    };
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| {
        let label = texts.format(text, &[("name", name.to_owned().into())]);
        WidgetInfo::labeled(Role::Button, true, &label)
    });
    let centre = pos2(rect.left() + CHEVRON / 2.0, rect.center().y);
    chevron(ui.painter(), centre, CHEVRON, expanded, palette::PRIMARY);
    if response.has_focus() {
        let mark = Rect::from_center_size(centre, vec2(CHEVRON, CHEVRON));
        focus_ring_square(ui, mark, palette::PRIMARY, FOCUS_OFFSET);
    }
    response.clicked()
}
