use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use egui::{Galley, Id, Ui};

use super::diff_line::display;
use crate::theme::desktop::{ROW_FONT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING};
use crate::theme::tokens::{Weight, font};
use crate::widgets::chip;
use crate::widgets::rows::row_weight;
use crate::widgets::text::{caps, line};
use crate::{LocalizationTab, LocalizationTable, LocalizationText, Text, TextLanguage, Texts};

const MAX_CACHED: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Key {
    serial: u64,
    language: crate::Language,
    pixels_per_point: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Heads {
    table: u64,
    language: crate::Language,
    pixels_per_point: u32,
}

/// What a cell shows.
pub(super) enum Cell {
    Missing,
    Text(Arc<Galley>),
}

type Named = (Arc<str>, Arc<Galley>);

struct Cache {
    key: Option<Key>,
    heads_key: Option<Heads>,
    heads: Vec<Named>,
    missing: Option<Named>,
    cells: HashMap<(u32, usize), Arc<Cell>>,
    active: Option<((u32, usize), Arc<Galley>)>,
}

/// The laid-out heads and cells of the table, kept until the shown rows, the table or the
/// interface language change; the heads until the table, the language or the scale change.
#[derive(Clone)]
pub(super) struct Laid(Arc<Mutex<Cache>>);

impl Laid {
    pub(super) fn get(
        ui: &Ui,
        texts: &Texts,
        tab: &LocalizationTab,
        table: &LocalizationTable,
    ) -> Self {
        let id = Id::unique("p4kx-localization-table");
        let laid = ui.data_mut(|data| {
            data.get_temp_mut_or_insert_with(id, || {
                Self(Arc::new(Mutex::new(Cache {
                    key: None,
                    heads_key: None,
                    heads: Vec::new(),
                    missing: None,
                    cells: HashMap::new(),
                    active: None,
                })))
            })
            .clone()
        });
        let pixels_per_point = ui.ctx().pixels_per_point().to_bits();
        let key = Key {
            serial: tab.serial(),
            language: texts.language(),
            pixels_per_point,
        };
        let heads_key = Heads {
            table: tab.table_serial(),
            language: key.language,
            pixels_per_point,
        };
        let mut cache = laid.lock();
        if cache.heads_key != Some(heads_key) {
            let missing = texts.get(LocalizationText::Missing);
            cache.heads_key = Some(heads_key);
            cache.heads = heads(ui, texts, table);
            cache.missing = Some((Arc::from(missing.as_str()), chip::galley(ui, &missing)));
            cache.key = None;
        }
        if cache.key != Some(key) || cache.cells.len() > MAX_CACHED {
            cache.key = Some(key);
            cache.cells.clear();
            cache.active = None;
        }
        drop(cache);
        laid
    }

    fn lock(&self) -> MutexGuard<'_, Cache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn cell(
        &self,
        ui: &Ui,
        table: &LocalizationTable,
        (row, column): (u32, usize),
    ) -> Arc<Cell> {
        if let Some(cell) = self.lock().cells.get(&(row, column)) {
            return Arc::clone(cell);
        }
        let text = text_of(table, row, column);
        let cell = Arc::new(text.map_or(Cell::Missing, |text| {
            Cell::Text(layout(ui, text, Weight::Regular))
        }));
        self.lock().cells.insert((row, column), Arc::clone(&cell));
        cell
    }

    pub(super) fn active(
        &self,
        ui: &Ui,
        table: &LocalizationTable,
        (row, column): (u32, usize),
    ) -> Option<Arc<Galley>> {
        if let Some((at, galley)) = &self.lock().active
            && *at == (row, column)
        {
            return Some(Arc::clone(galley));
        }
        let galley = layout(ui, text_of(table, row, column)?, row_weight(true));
        self.lock().active = Some(((row, column), Arc::clone(&galley)));
        Some(galley)
    }

    pub(super) fn head(&self, column: usize) -> Option<Named> {
        self.lock().heads.get(column).cloned()
    }

    pub(super) fn missing(&self) -> Option<Named> {
        self.lock().missing.clone()
    }
}

fn layout(ui: &Ui, text: &str, weight: Weight) -> Arc<Galley> {
    let job = display(text.as_bytes()).job(&font(ROW_FONT, weight));
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

fn heads(ui: &Ui, texts: &Texts, table: &LocalizationTable) -> Vec<Named> {
    std::iter::once(texts.get(LocalizationText::ColumnKey))
        .chain(
            table
                .languages()
                .iter()
                .map(|&language| language_name(texts, language)),
        )
        .map(|name| {
            let text = caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
            (Arc::from(name.as_str()), line(ui, text, f32::INFINITY))
        })
        .collect()
}

/// Returns the name of `language` in the interface language.
pub(super) fn language_name(texts: &Texts, language: TextLanguage) -> String {
    let known = LocalizationText::LanguageName(language);
    if known.id() == LocalizationText::OtherLanguage.id() {
        return format!(
            "{} ({})",
            texts.get(LocalizationText::OtherLanguage),
            language.code()
        );
    }
    texts.get(Text::Localization(known))
}

/// Returns the text of the cell of table row `id` in `column`: the key for column 0, else the
/// text in the language of the column, or `None` when it has none.
pub(super) fn text_of(table: &LocalizationTable, id: u32, column: usize) -> Option<&str> {
    column.checked_sub(1).map_or_else(
        || table.key(id),
        |index| {
            table
                .languages()
                .get(index)
                .and_then(|&language| table.text(id, language))
        },
    )
}
