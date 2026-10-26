// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::Ui;

use super::sound_grid::{self, Cell, Grid, Laid, Row, Tint, Width};
use crate::theme::sound_metrics::{SOUND_NAME_COLUMN, SOUND_STATE_COLUMN};
use crate::{Language, SoundsText, Texts, TriggerList, TriggerRow, Unmatched};

const WIDTHS: [Width; 3] = [
    Width::Initial(SOUND_NAME_COLUMN),
    Width::Initial(SOUND_STATE_COLUMN),
    Width::Rest,
];

fn state(texts: &Texts, row: &TriggerRow) -> Cell {
    match row.unmatched {
        None => Cell::new(texts.get(SoundsText::StateOk), Tint::Success),
        Some(Unmatched::NoDefinition) => {
            Cell::new(texts.get(SoundsText::StateNoDefinition), Tint::Warning)
        }
        Some(Unmatched::NoEvent) => Cell::new(texts.get(SoundsText::StateNoEvent), Tint::Warning),
        Some(_) => Cell::new(texts.get(SoundsText::StateNoSoundbank), Tint::Warning),
    }
}

fn lay(texts: &Texts, list: &TriggerList) -> Laid {
    let rows = list
        .rows
        .iter()
        .map(|row| {
            let events = row
                .events
                .iter()
                .map(|event| event.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Row {
                name: row.name.clone(),
                cells: vec![
                    Cell::new(row.name.clone(), Tint::Normal),
                    state(texts, row),
                    Cell::new(events, Tint::Muted),
                ],
            }
        })
        .collect();
    Laid {
        heads: vec![
            texts.get(SoundsText::ColumnTrigger),
            texts.get(SoundsText::ColumnState),
            texts.get(SoundsText::ColumnEvents),
        ],
        rows,
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Key {
    serial: u64,
    language: Language,
}

/// Shows the triggers of `list` as a table of trigger, status and Wwise events, `selected`
/// highlighted; returns the position of the row that was clicked.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    (list, serial): (&TriggerList, u64),
    selected: Option<usize>,
) -> Option<usize> {
    let key = Key {
        serial,
        language: texts.language(),
    };
    let laid = sound_grid::cached(ui, "sound-triggers", key, || lay(texts, list));
    let grid = Grid {
        salt: "sound-triggers",
        label: texts.get(SoundsText::ScrollTriggers),
        widths: &WIDTHS,
        laid: &laid,
        selected,
        height: None,
    };
    sound_grid::show(ui, &grid)
}
