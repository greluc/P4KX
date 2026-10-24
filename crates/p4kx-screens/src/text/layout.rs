// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ops::Range;

use p4kx_gfx::FontFace;

const SPACE_ADVANCE: f32 = 0.3;
const TOFU_ADVANCE: f32 = 0.6;
const GLYPH_FALLBACK_ADVANCE: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cell {
    pub(crate) ch: char,
    pub(crate) glyph: Option<usize>,
    pub(crate) advance: f32,
}

impl Cell {
    pub(crate) const fn is_newline(&self) -> bool {
        self.ch == '\n'
    }

    pub(crate) const fn is_space(&self) -> bool {
        self.ch == ' '
    }

    pub(crate) const fn is_visible(&self) -> bool {
        !self.is_newline() && !self.is_space()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Shaping {
    pub(crate) size: f32,
    pub(crate) letter_spacing: f32,
    pub(crate) kerning: bool,
}

pub(crate) fn shape(face: &FontFace<'_>, text: &str, how: Shaping, out: &mut Vec<Cell>) {
    let mut previous: Option<char> = None;
    for ch in text.chars() {
        let ch = match ch {
            '\r' => continue,
            '\t' | '\u{a0}' => ' ',
            other => other,
        };
        if ch == '\n' {
            out.push(Cell {
                ch,
                glyph: None,
                advance: 0.0,
            });
            previous = None;
            continue;
        }
        let glyph = face.glyph(ch);
        let advance = glyph
            .and_then(|glyph| face.advance(glyph))
            .map(|advance| advance * how.size);
        let advance = match (ch, glyph, advance) {
            (_, _, Some(advance)) => advance,
            (' ', _, None) => SPACE_ADVANCE * how.size,
            (_, Some(_), None) => GLYPH_FALLBACK_ADVANCE * how.size,
            (_, None, None) => TOFU_ADVANCE * how.size,
        };
        if how.kerning
            && let Some(left) = previous
            && let Some(last) = out.last_mut()
        {
            last.advance = face.kerning(left, ch).mul_add(how.size, last.advance);
        }
        out.push(Cell {
            ch,
            glyph,
            advance: advance + how.letter_spacing,
        });
        previous = Some(ch);
    }
}

pub(crate) fn lines(cells: &[Cell], wrap: Option<f32>) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let (mut start, mut at, mut width) = (0, 0, 0.0_f32);
    let mut space_after: Option<usize> = None;
    while let Some(cell) = cells.get(at) {
        if cell.is_newline() {
            lines.push(start..at);
            start = at + 1;
            (at, width, space_after) = (start, 0.0, None);
            continue;
        }
        if let Some(limit) = wrap
            && at > start
            && !cell.is_space()
            && width + cell.advance > limit
        {
            let end = space_after.filter(|&end| end > start).unwrap_or(at);
            lines.push(start..end);
            (start, at, width, space_after) = (end, end, 0.0, None);
            continue;
        }
        width += cell.advance;
        if cell.is_space() {
            space_after = Some(at + 1);
        }
        at += 1;
    }
    if start < cells.len() || lines.is_empty() {
        lines.push(start..cells.len());
    }
    lines
}

pub(crate) fn width(cells: &[Cell]) -> f32 {
    let trailing = cells
        .iter()
        .rev()
        .take_while(|cell| cell.is_space())
        .count();
    cells
        .get(..cells.len() - trailing)
        .unwrap_or_default()
        .iter()
        .map(|cell| cell.advance)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{Cell, lines, width};

    fn cells(text: &str) -> Vec<Cell> {
        text.chars()
            .map(|ch| Cell {
                ch,
                glyph: Some(0),
                advance: 4.0,
            })
            .collect()
    }

    #[test]
    fn a_text_without_wrapping_breaks_at_newlines_only() {
        let cells = cells("ab\ncd ef");
        assert_eq!(lines(&cells, None), [0..2, 3..8]);
    }

    #[test]
    fn words_wrap_after_the_last_space_that_fits() {
        let cells = cells("ab cd ef");
        assert_eq!(lines(&cells, Some(14.0)), [0..3, 3..6, 6..8]);
    }

    #[test]
    fn a_word_longer_than_the_line_is_broken() {
        let cells = cells("abcdefgh");
        assert_eq!(lines(&cells, Some(12.0)), [0..3, 3..6, 6..8]);
    }

    #[test]
    fn a_text_ending_in_a_newline_has_no_extra_line_and_an_empty_text_has_one() {
        let trailing = lines(&cells("ab\n"), None);
        assert_eq!((trailing.len(), trailing.first()), (1, Some(&(0..2))));
        let empty = lines(&[], None);
        assert_eq!((empty.len(), empty.first()), (1, Some(&(0..0))));
    }

    #[test]
    fn trailing_spaces_do_not_count_to_the_width() {
        let cells = cells("ab  ");
        assert!((width(&cells) - 8.0).abs() < f32::EPSILON);
    }
}
