// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, Galley, Id, Rect, Ui};
use p4kx_comparison::RecordChangeKind;

use super::moved_path::MovedPath;
use crate::comparison::kind_rank;
use crate::theme::desktop::ROW_FONT;
use crate::theme::tokens::Weight;
use crate::widgets::chip::{self, Tone};
use crate::widgets::text::{line, paint_left, plain};
use crate::{ComparisonText, RecordChangeRow, RecordChangeText, RecordChangeView, Text, Texts};

const MAX_ROWS: usize = 512;

/// The kinds of Record change in list order, with their text and chip tone.
const KINDS: [(Text, Tone); 5] = [
    (Text::Comparison(ComparisonText::Added), Tone::Success),
    (Text::Comparison(ComparisonText::Removed), Tone::Danger),
    (Text::RecordChange(RecordChangeText::Moved), Tone::Info),
    (Text::Comparison(ComparisonText::Changed), Tone::Warning),
    (
        Text::RecordChange(RecordChangeText::Unreadable),
        Tone::Muted,
    ),
];

/// Returns the text and the chip tone of `kind`.
pub(super) fn kind_look(kind: RecordChangeKind) -> (Text, Tone) {
    KINDS.get(usize::from(kind_rank(kind))).copied().unwrap_or((
        Text::RecordChange(RecordChangeText::Unreadable),
        Tone::Muted,
    ))
}

/// Returns the old and the new record path of a moved Record, `None` for another change.
pub(super) fn moved(row: &RecordChangeRow) -> Option<(&str, &str)> {
    match (row.kind(), row.old_path(), row.new_path()) {
        (RecordChangeKind::Moved, Some(old), Some(new)) => Some((old, new)),
        _ => None,
    }
}

/// Returns the record path of `row` as words: both paths for a moved Record.
pub(super) fn shown_path(texts: &Texts, row: &RecordChangeRow) -> String {
    match (row.kind(), row.old_path(), row.new_path()) {
        (RecordChangeKind::Moved, Some(old), Some(new)) => texts.format(
            RecordChangeText::MovedPath,
            &[
                ("old", old.to_owned().into()),
                ("new", new.to_owned().into()),
            ],
        ),
        _ => row.path().to_owned(),
    }
}

/// A text cell of a Record row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Cell {
    Path,
    Type,
}

/// The laid-out text of a cell: one line, or both record paths of a moved Record.
#[derive(Clone)]
pub(super) enum CellText {
    Line(Arc<Galley>),
    Moved(MovedPath),
}

impl CellText {
    /// Returns the width it takes.
    pub(super) fn width(&self) -> f32 {
        match self {
            Self::Line(galley) => galley.size().x,
            Self::Moved(paths) => paths.width(),
        }
    }

    /// Paints the text left-aligned and vertically centred in `rect` in `colour`.
    pub(super) fn paint(&self, ui: &Ui, rect: Rect, colour: Color32) {
        match self {
            Self::Line(galley) => paint_left(ui, rect, Arc::clone(galley), colour),
            Self::Moved(paths) => paths.paint(ui, rect.left_center(), colour),
        }
    }
}

/// The laid-out text of one cell.
#[derive(Clone)]
struct Laid {
    width: f32,
    weight: Weight,
    text: CellText,
}

/// The laid-out texts of the Record rows shown recently and of the chips, kept across frames
/// until the Comparison or the language changes.
#[derive(Clone, Default)]
pub(super) struct RecordRowCache {
    key: Option<Id>,
    chips: Vec<Arc<Galley>>,
    heads: Vec<String>,
    rows: HashMap<(u32, Cell), Laid>,
}

impl RecordRowCache {
    /// Returns the cache of `ui` for the Comparison `generation` in the language of `texts`,
    /// emptied when either changed.
    pub(super) fn load(ui: &Ui, texts: &Texts, generation: u64, heads: &[Text]) -> Self {
        let key = Id::unique((
            texts.language(),
            generation,
            ui.ctx().pixels_per_point().to_bits(),
        ));
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(cache_id())));
        if cache.key != Some(key) {
            cache = Self {
                key: Some(key),
                chips: KINDS
                    .map(|(text, _)| chip::galley(ui, &texts.get(text)))
                    .to_vec(),
                heads: heads.iter().map(|&text| texts.get(text)).collect(),
                rows: HashMap::new(),
            };
        }
        cache
    }

    /// Keeps the cache in `ui` for the next frame.
    pub(super) fn store(self, ui: &Ui) {
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(cache_id()) = self);
    }

    /// Returns the label of the head cell at `index`.
    pub(super) fn head(&self, index: usize) -> &str {
        self.heads.get(index).map_or("", String::as_str)
    }

    /// Returns the chip label of `kind`.
    pub(super) fn chip(&self, kind: RecordChangeKind) -> Option<Arc<Galley>> {
        self.chips.get(usize::from(kind_rank(kind))).cloned()
    }

    /// Returns the text of `cell` of row `id`, laid out `width` wide in `weight`.
    pub(super) fn cell(
        &mut self,
        ui: &Ui,
        (id, row, cell): (u32, &RecordChangeRow, Cell),
        (width, weight): (f32, Weight),
    ) -> CellText {
        if let Some(kept) = self.rows.get(&(id, cell))
            && (kept.width - width).abs() < 0.5
            && kept.weight == weight
        {
            return kept.text.clone();
        }
        if self.rows.len() >= MAX_ROWS {
            self.rows.clear();
        }
        let text = match (cell, moved(row)) {
            (Cell::Path, Some(paths)) => {
                CellText::Moved(MovedPath::lay_out(ui, paths, (ROW_FONT, weight), width))
            }
            (Cell::Path, None) => {
                CellText::Line(line(ui, plain(row.path(), ROW_FONT, weight), width))
            }
            (Cell::Type, _) => {
                CellText::Line(line(ui, plain(row.type_name(), ROW_FONT, weight), width))
            }
        };
        self.rows.insert(
            (id, cell),
            Laid {
                width,
                weight,
                text: text.clone(),
            },
        );
        text
    }
}

/// Returns the row `id` of `view` when it is shown.
pub(super) fn shown_row(view: &RecordChangeView, index: usize) -> Option<(u32, &RecordChangeRow)> {
    let id = *view.shown().get(index)?;
    Some((id, view.row(id)?))
}

fn cache_id() -> Id {
    Id::unique("comparison-record-row-cache")
}
