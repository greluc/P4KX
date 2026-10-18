use egui::collapsing_header::paint_default_icon;
use egui::{Role, ScrollArea, Sense, Ui, WidgetInfo, vec2};

use super::row_height;
use crate::{Browser, FolderId, Model, Text, Texts, TreeRow};

const INDENT: f32 = 14.0;
const TOGGLE_WIDTH: f32 = 18.0;

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
    ScrollArea::both().auto_shrink(false).show_rows(
        ui,
        row_height(ui),
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
    let name = if row.folder == index.root() {
        index
            .archive()
            .path()
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    } else {
        folder.name().to_owned()
    };
    ui.horizontal(|ui| {
        ui.add_space(INDENT * depth(row.depth));
        let mut action = None;
        if folder.children().is_empty() {
            ui.add_space(TOGGLE_WIDTH);
        } else if toggle(ui, texts, browser.is_expanded(row.folder), &name) {
            action = Some(Action::Toggle(row.folder));
        }
        let selected = browser.selected_folder() == row.folder;
        if ui.selectable_label(selected, &name).clicked() {
            action = Some(Action::Select(row.folder));
        }
        action
    })
    .inner
}

fn toggle(ui: &mut Ui, texts: &Texts, expanded: bool, name: &str) -> bool {
    let (openness, text) = if expanded {
        (1.0, Text::CollapseFolder)
    } else {
        (0.0, Text::ExpandFolder)
    };
    let label = texts.format(text, &[("name", name.to_owned().into())]);
    let (_, response) = ui.allocate_exact_size(vec2(TOGGLE_WIDTH, row_height(ui)), Sense::click());
    paint_default_icon(ui, openness, &response);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
    response.clicked()
}

fn depth(depth: usize) -> f32 {
    f32::from(u16::try_from(depth).unwrap_or(u16::MAX))
}
