// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::FontFace;

use super::layout::{self, Cell, Shaping};

pub(super) const MIN_SIZE: f32 = 1.0;

/// How a text was fitted to the width of its box: the width asked for, the width of its widest
/// line at the size asked for, and whether the size had to shrink.
#[derive(Debug)]
pub(super) struct Fit {
    pub(super) width: f32,
    pub(super) widest: f32,
    pub(super) shrunk: bool,
}

/// Shapes `text`; with `fit_width` the size shrinks until the widest line fits it.
pub(super) fn shape_fitted(
    face: &FontFace<'_>,
    text: &str,
    how: Shaping,
    fit_width: Option<f32>,
) -> (Vec<Cell>, f32, Option<Fit>) {
    let mut cells = Vec::new();
    layout::shape(face, text, how, &mut cells);
    let Some(width) = fit_width.filter(|width| *width > 0.0) else {
        return (cells, how.size, None);
    };
    let widest = layout::lines(&cells, None)
        .into_iter()
        .filter_map(|line| cells.get(line))
        .map(layout::width)
        .fold(0.0_f32, f32::max);
    let shrunk = widest > width;
    let fit = Fit {
        width,
        widest,
        shrunk,
    };
    if !shrunk {
        return (cells, how.size, Some(fit));
    }
    let size = (how.size * width / widest).max(MIN_SIZE);
    cells.clear();
    layout::shape(face, text, Shaping { size, ..how }, &mut cells);
    (cells, size, Some(fit))
}
