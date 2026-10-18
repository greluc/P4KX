use std::collections::HashMap;
use std::sync::Arc;

use egui::{Galley, Id, Ui};
use p4kx_comparison::{ChangeKind, Entry, EntryChange};

use crate::theme::desktop::ROW_FONT;
use crate::theme::tokens::Weight;
use crate::widgets::chip;
use crate::widgets::text::{line, plain};
use crate::{ComparisonText, ComparisonView, Texts, format};

const MAX_ROWS: usize = 512;

/// The laid-out texts of one change row.
#[derive(Clone)]
pub(super) struct RowGalleys {
    width: f32,
    weight: Weight,
    pub(super) path: Arc<Galley>,
    pub(super) old: Arc<Galley>,
    pub(super) new: Arc<Galley>,
}

/// The laid-out texts of the rows shown recently and of the change chips, kept across frames
/// until the Comparison or the language changes.
#[derive(Clone, Default)]
pub(super) struct RowCache {
    key: Option<Id>,
    chips: Vec<Arc<Galley>>,
    rows: HashMap<u32, RowGalleys>,
}

impl RowCache {
    /// Returns the cache of `ui` for `view` in the language of `texts`, emptied when either
    /// changed.
    pub(super) fn load(ui: &Ui, texts: &Texts, view: &ComparisonView) -> Self {
        let key = Id::unique((
            texts.language(),
            std::ptr::from_ref(view.comparison()).addr(),
            ui.ctx().pixels_per_point().to_bits(),
        ));
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(cache_id())));
        if cache.key != Some(key) {
            cache = Self {
                key: Some(key),
                chips: [
                    ComparisonText::Added,
                    ComparisonText::Removed,
                    ComparisonText::Changed,
                ]
                .map(|text| chip::galley(ui, &texts.get(text)))
                .to_vec(),
                rows: HashMap::new(),
            };
        }
        cache
    }

    /// Keeps the cache in `ui` for the next frame.
    pub(super) fn store(self, ui: &Ui) {
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(cache_id()) = self);
    }

    /// Returns the chip label of `kind`.
    pub(super) fn chip(&self, kind: ChangeKind) -> Option<Arc<Galley>> {
        let index = match kind {
            ChangeKind::Added => 0,
            ChangeKind::Removed => 1,
            ChangeKind::Changed => 2,
        };
        self.chips.get(index).cloned()
    }

    /// Returns the texts of the row of change `id`, laid out for a path cell `width` wide in
    /// `weight`.
    pub(super) fn row(
        &mut self,
        ui: &Ui,
        texts: &Texts,
        (id, change): (u32, &EntryChange<'_>),
        (width, weight): (f32, Weight),
    ) -> RowGalleys {
        if let Some(row) = self.rows.get(&id)
            && (row.width - width).abs() < 0.5
            && row.weight == weight
        {
            return row.clone();
        }
        if self.rows.len() >= MAX_ROWS {
            self.rows.clear();
        }
        let size = |entry: Option<&Entry>| {
            entry.map_or_else(
                || texts.get(ComparisonText::NoSize),
                |entry| format::size(entry.uncompressed_size(), texts.language()),
            )
        };
        let row = RowGalleys {
            width,
            weight,
            path: line(ui, plain(change.path(), ROW_FONT, weight), width),
            old: line(
                ui,
                plain(size(change.old_entry()), ROW_FONT, weight),
                f32::INFINITY,
            ),
            new: line(
                ui,
                plain(size(change.new_entry()), ROW_FONT, weight),
                f32::INFINITY,
            ),
        };
        self.rows.insert(id, row.clone());
        row
    }
}

fn cache_id() -> Id {
    Id::unique("comparison-row-cache")
}
