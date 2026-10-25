// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::Serialize;

const MAX_EXAMPLES: usize = 3;

/// An element type or feature the render left out or approximated, with how often.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unsupported {
    /// The element type, e.g. `WidgetText`, without the `BuildingBlocks_` prefix.
    pub element: String,
    /// What was left out or approximated.
    pub reason: String,
    /// How many widgets it concerns.
    pub count: usize,
    /// Names of up to three of those widgets.
    pub examples: Vec<String>,
}

/// What a render drew and what it left out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The widgets visited, every nested canvas included.
    pub widgets: usize,
    /// The shapes drawn, every placement of a glyph counted.
    pub shapes: usize,
    /// The text widgets whose text was drawn.
    pub texts_drawn: usize,
    /// The text widgets whose text was not drawn: no fonts were given, the text is empty or filled
    /// in by the game at run time, the font or colour is missing, or a limit was reached.
    pub texts_left_out: usize,
    /// The widgets whose texture or vector graphic was drawn.
    pub images_drawn: usize,
    /// The widgets whose texture or vector graphic was not drawn: no image sources were given,
    /// the file is missing, unreadable or beyond a limit, or the widget names none.
    pub images_left_out: usize,
    /// The canvases composed, the screen's own first.
    pub canvases: usize,
    /// The element types and features left out or approximated.
    pub unsupported: Vec<Unsupported>,
    /// Record paths of nested canvases that could not be read.
    pub missing_canvases: Vec<String>,
    /// Whether a limit stopped the composition before the whole screen was drawn.
    pub truncated: bool,
}

impl Report {
    pub(crate) fn note(&mut self, element: &str, reason: &str, name: &str) {
        let element = element.strip_prefix("BuildingBlocks_").unwrap_or(element);
        if let Some(entry) = self
            .unsupported
            .iter_mut()
            .find(|entry| entry.element == element && entry.reason == reason)
        {
            entry.count += 1;
            if entry.examples.len() < MAX_EXAMPLES && !entry.examples.iter().any(|n| n == name) {
                entry.examples.push(name.to_owned());
            }
            return;
        }
        self.unsupported.push(Unsupported {
            element: element.to_owned(),
            reason: reason.to_owned(),
            count: 1,
            examples: vec![name.to_owned()],
        });
    }

    /// Returns whether the whole screen was drawn with every effect.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.unsupported.is_empty() && self.missing_canvases.is_empty() && !self.truncated
    }
}
