use std::collections::HashMap;
use std::sync::Arc;

use egui::{Galley, Id, Ui};
use p4kx_comparison::RecordChangeKind;

use crate::comparison::kind_rank;
use crate::theme::desktop::ROW_FONT;
use crate::theme::tokens::Weight;
use crate::widgets::chip::{self, Tone};
use crate::widgets::text::{line, plain};
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

/// Returns the record path of `row` as the list shows it: both paths for a moved Record.
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

/// The laid-out text of one cell.
#[derive(Clone)]
struct Laid {
    width: f32,
    weight: Weight,
    galley: Arc<Galley>,
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
        texts: &Texts,
        (id, row, cell): (u32, &RecordChangeRow, Cell),
        (width, weight): (f32, Weight),
    ) -> Arc<Galley> {
        if let Some(kept) = self.rows.get(&(id, cell))
            && (kept.width - width).abs() < 0.5
            && kept.weight == weight
        {
            return Arc::clone(&kept.galley);
        }
        if self.rows.len() >= MAX_ROWS {
            self.rows.clear();
        }
        let text = match cell {
            Cell::Path => shown_path(texts, row),
            Cell::Type => row.type_name().to_owned(),
        };
        let galley = line(ui, plain(text, ROW_FONT, weight), width);
        self.rows.insert(
            (id, cell),
            Laid {
                width,
                weight,
                galley: Arc::clone(&galley),
            },
        );
        galley
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
