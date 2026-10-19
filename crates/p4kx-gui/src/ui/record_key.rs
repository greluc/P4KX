use std::sync::Arc;

use egui::text::TextWrapping;
use egui::{Galley, Rect, Ui, pos2};

use crate::shown_line::escaped_job;
use crate::theme::desktop::ROW_FONT;
use crate::theme::palette;
use crate::theme::tokens::{Weight, font};
use crate::widgets::chip::{self, Tone};
use crate::widgets::text::{line, paint_left, plain};
use crate::{KeyText, LocalizationText, Texts};

/// What follows the value of a line of the Record preview in its cell.
#[derive(Clone)]
pub(super) enum Trailer {
    /// The type of a Reference's target, muted.
    Type(Arc<Galley>),
    /// The text of a localisation key.
    Text(Arc<Galley>),
    /// The mark of a key without a text.
    Missing(Arc<Galley>),
}

impl Trailer {
    /// Lays out the type of a Reference's target in `room` points.
    pub(super) fn type_name(ui: &Ui, name: &str, room: f32) -> Self {
        Self::Type(line(ui, plain(name, ROW_FONT, Weight::Regular), room))
    }

    /// Lays out what follows a localisation key: its text in `room` points, or the missing mark;
    /// `None` when the key has no text to show.
    pub(super) fn key(ui: &Ui, texts: &Texts, text: &KeyText, room: f32) -> Option<Self> {
        match text {
            KeyText::None => None,
            KeyText::Text { text, escapes } => {
                let mut job = escaped_job(text, escapes, &font(ROW_FONT, Weight::Regular));
                job.wrap = TextWrapping::truncate_at_width(room);
                Some(Self::Text(ui.fonts_mut(|fonts| fonts.layout_job(job))))
            }
            KeyText::Missing => Some(Self::Missing(chip::galley(
                ui,
                &texts.get(LocalizationText::Missing),
            ))),
        }
    }

    /// Paints the trailer in `cell`, starting at its left edge.
    pub(super) fn paint(&self, ui: &Ui, cell: Rect) {
        match self {
            Self::Type(galley) => paint_left(ui, cell, Arc::clone(galley), palette::GRAY_2_TEXT),
            Self::Text(galley) => paint_left(ui, cell, Arc::clone(galley), palette::GRAY_1),
            Self::Missing(galley) => {
                chip::paint_clipped(
                    ui,
                    pos2(cell.left(), cell.center().y),
                    Arc::clone(galley),
                    Tone::Warning,
                    cell,
                );
            }
        }
    }
}
