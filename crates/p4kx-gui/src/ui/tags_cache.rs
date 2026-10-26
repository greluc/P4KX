// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use egui::{Galley, Id, Ui};

use super::search::eyebrow;
use crate::theme::desktop::{TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING};
use crate::theme::tokens::Weight;
use crate::widgets::text::{caps, line};
use crate::{TagsText, Texts};

/// A text and the galley it was laid out as on one line.
#[derive(Clone)]
pub(super) struct Laid {
    pub(super) text: Arc<str>,
    pub(super) galley: Arc<Galley>,
}

/// The texts of the Tags tab that only the language changes: the toolbar and the heads.
#[derive(Clone)]
pub(super) struct Labels {
    pub(super) eyebrow: Laid,
    pub(super) hint: Arc<str>,
    pub(super) copy_path: Arc<str>,
    pub(super) copy_guid: Arc<str>,
    pub(super) heads: [(TagsText, Laid); 3],
}

/// A value laid out for a cell that was this many points wide, as the bits of the width.
pub(super) type ForWidth<T> = Option<(u32, T)>;

/// What a row of the tree has laid out so far; a cell is laid out when it first comes into view,
/// and again when its width changes.
#[derive(Default)]
pub(super) struct RowCells {
    pub(super) name: ForWidth<Arc<Galley>>,
    pub(super) name_bold: ForWidth<Arc<Galley>>,
    pub(super) guid: ForWidth<Laid>,
    pub(super) legacy: ForWidth<Laid>,
}

/// What the Tags tab keeps from frame to frame: its texts, the footer and warning texts, and the
/// cells of the rows. Made again when the language or the scale changes; the cells also when
/// another tree is shown.
pub(super) struct Cache {
    key: u64,
    labels: Labels,
    generation: u64,
    rows: Vec<RowCells>,
    pub(super) count: Option<((usize, usize), Laid)>,
    pub(super) irregular: Option<(usize, Arc<str>)>,
}

/// The heads of the columns, left to right.
pub(super) const HEADS: [TagsText; 3] = [
    TagsText::ColumnTag,
    TagsText::ColumnGuid,
    TagsText::ColumnLegacy,
];

impl Cache {
    /// Returns the cache of the tab, made again unless it fits the language and the scale.
    pub(super) fn get(ui: &Ui, texts: &Texts) -> Arc<Mutex<Self>> {
        let mut hasher = DefaultHasher::new();
        texts.language().hash(&mut hasher);
        ui.ctx().pixels_per_point().to_bits().hash(&mut hasher);
        let key = hasher.finish();
        let id = Id::NULL;
        if let Some(cache) = ui.data(|data| data.get_temp::<Arc<Mutex<Self>>>(id))
            && Self::lock(&cache).key == key
        {
            return cache;
        }
        let cache = Arc::new(Mutex::new(Self {
            key,
            labels: labels(ui, texts),
            generation: 0,
            rows: Vec::new(),
            count: None,
            irregular: None,
        }));
        ui.data_mut(|data| data.insert_temp(id, Arc::clone(&cache)));
        cache
    }

    /// Locks the cache.
    pub(super) fn lock(cache: &Mutex<Self>) -> MutexGuard<'_, Self> {
        cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Returns the texts of the tab.
    pub(super) fn labels(&self) -> Labels {
        self.labels.clone()
    }

    /// Returns the cells of the `len` rows of the tree read as `generation`, forgetting them when
    /// that is another tree than before.
    pub(super) fn rows(&mut self, generation: u64, len: usize) -> &mut [RowCells] {
        if self.generation != generation || self.rows.len() != len {
            self.generation = generation;
            self.rows.clear();
            self.rows.resize_with(len, RowCells::default);
        }
        &mut self.rows
    }
}

/// Returns the value in `slot` when it was made for `key`, and otherwise makes it with `make`.
pub(super) fn memo<K: PartialEq, V>(
    slot: &mut Option<(K, V)>,
    key: K,
    make: impl FnOnce() -> V,
) -> &V {
    let entry = match slot.take() {
        Some(entry) if entry.0 == key => entry,
        _ => (key, make()),
    };
    &slot.insert(entry).1
}

fn labels(ui: &Ui, texts: &Texts) -> Labels {
    let lay = |text: String, rich: egui::RichText| Laid {
        galley: line(ui, rich, f32::INFINITY),
        text: Arc::from(text),
    };
    let label = texts.get(TagsText::Label);
    let title = lay(label.clone(), eyebrow(&label));
    let heads = HEADS.map(|head| {
        let name = texts.get(head);
        let rich = caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
        (head, lay(name, rich))
    });
    Labels {
        eyebrow: title,
        hint: Arc::from(texts.get(TagsText::Hint)),
        copy_path: Arc::from(texts.get(TagsText::CopyPath)),
        copy_guid: Arc::from(texts.get(TagsText::CopyGuid)),
        heads,
    }
}
