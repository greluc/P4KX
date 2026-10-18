use std::f32::consts::PI;

use egui::{Align, Button, Checkbox, Layout, Role, Sense, Shape, Ui, WidgetInfo, vec2};
use egui_extras::{Column, TableBuilder, TableRow};
use p4kx_extraction::{Compression, Entry};

use super::row_height;
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
}

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let height = row_height(ui);
    let mut action = None;
    TableBuilder::new(ui)
        .id_salt("entries")
        .striped(true)
        .resizable(true)
        .auto_shrink(false)
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::exact(height))
        .column(Column::initial(320.0).at_least(120.0).clip(true))
        .columns(Column::initial(110.0).at_least(60.0).clip(true), 2)
        .column(Column::remainder().at_least(80.0))
        .header(height, |mut header| {
            header.col(|ui| {
                let mut all = browser.all_shown_checked();
                let label = texts.get(Text::SelectAll);
                if labelled_checkbox(ui, &mut all, &label) {
                    action = Some(Action::CheckShown(all));
                }
            });
            for (column, text) in COLUMNS {
                header.col(|ui| {
                    if sort_header(ui, browser, column, &texts.get(text)) {
                        action = Some(Action::Sort(column));
                    }
                });
            }
        })
        .body(|body| {
            body.rows(height, browser.shown().len(), |row| {
                if let Some(clicked) = show_row(row, texts, browser) {
                    action = Some(clicked);
                }
            });
        });
    let ctx = ui.ctx().clone();
    if let (Some(action), Some(browser)) = (action, model.browser_mut()) {
        match action {
            Action::Sort(column) => browser.sort_by(column),
            Action::CheckShown(checked) => browser.set_all_shown_checked(checked),
            Action::Check(id, checked) => browser.set_checked(id, checked),
            Action::Preview(id) => browser.select_entry(id, move || ctx.request_repaint()),
        }
    }
}

fn show_row(mut row: TableRow<'_, '_>, texts: &Texts, browser: &Browser) -> Option<Action> {
    let id = *browser.shown().get(row.index())?;
    let entry = browser.index().entry(id)?;
    let name = browser.index().name(id);
    let language = texts.language();
    let mut action = None;
    row.col(|ui| {
        let mut checked = browser.is_checked(id);
        let label = texts.format(Text::SelectEntry, &[("name", name.to_owned().into())]);
        if labelled_checkbox(ui, &mut checked, &label) {
            action = Some(Action::Check(id, checked));
        }
    });
    row.col(|ui| {
        let selected = browser.selected_entry() == Some(id);
        let response = ui.add(Button::selectable(selected, name).truncate());
        if response.on_hover_text(entry.path()).clicked() {
            action = Some(Action::Preview(id));
        }
    });
    row.col(|ui| {
        ui.label(compression(texts, entry));
    });
    row.col(|ui| {
        let text = if entry.is_encrypted() {
            Text::EncryptedYes
        } else {
            Text::EncryptedNo
        };
        ui.label(texts.get(text));
    });
    row.col(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(format::size(entry.uncompressed_size(), language));
        });
    });
    action
}

fn labelled_checkbox(ui: &mut Ui, checked: &mut bool, label: &str) -> bool {
    let response = ui.add(Checkbox::without_text(checked));
    let state = *checked;
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, state, label));
    response.changed()
}

fn sort_header(ui: &mut Ui, browser: &Browser, column: SortColumn, label: &str) -> bool {
    let clicked = ui.add(Button::new(label).frame(false)).clicked();
    let sort = browser.sort();
    if sort.column == column {
        let size = ui.spacing().icon_width * 0.6;
        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        let rotation = match sort.direction {
            SortDirection::Ascending => PI,
            SortDirection::Descending => 0.0,
        };
        let color = ui.visuals().text_color();
        ui.painter()
            .add(Shape::rotated_triangle(rect, rotation, color));
    }
    clicked
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
