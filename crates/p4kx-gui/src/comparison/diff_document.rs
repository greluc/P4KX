// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ops::Range;

use p4kx_comparison::{LineKind, TextDiff};

use super::ComparisonSide;

const CANCEL_STRIDE: u32 = 4096;

/// One line of one side in a row of the side-by-side diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiffCell {
    /// The zero-based line on its side.
    pub line: u32,
    /// Whether the line is on both sides, removed or added.
    pub kind: LineKind,
}

/// One row of the side-by-side diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffRow {
    /// The head of the hunk with this index.
    Hunk(u32),
    /// A line of each side; a side without a line at this row is `None`.
    Lines {
        /// The line of the old side.
        old: Option<DiffCell>,
        /// The line of the new side.
        new: Option<DiffCell>,
    },
}

/// One hunk of the side-by-side diff: where its head row is and the lines it covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffHunk {
    /// The row of its head.
    pub row: u32,
    /// The zero-based lines it covers on the old side.
    pub old: Range<u32>,
    /// The zero-based lines it covers on the new side.
    pub new: Range<u32>,
}

/// The line diff of one Entry laid out side by side: hunk heads, unchanged lines on both sides,
/// removed lines on the left beside the added lines that replace them.
#[derive(Debug)]
pub struct DiffDocument {
    text: TextDiff,
    rows: Vec<DiffRow>,
    hunks: Vec<DiffHunk>,
    widest: (ComparisonSide, u32),
}

impl DiffDocument {
    /// Lays out `text` side by side, or returns `None` once `cancelled` holds.
    pub(crate) fn build(text: TextDiff, cancelled: &dyn Fn() -> bool) -> Option<Self> {
        let mut builder = Builder::default();
        for (index, hunk) in text.hunks().enumerate() {
            if cancelled() {
                return None;
            }
            let (old, new) = (span(hunk.old_lines()), span(hunk.new_lines()));
            builder.hunks.push(DiffHunk {
                row: count(builder.rows.len()),
                old: old.clone(),
                new: new.clone(),
            });
            builder.rows.push(DiffRow::Hunk(count(index)));
            builder.at = (old.start, new.start);
            for (step, line) in (1..).zip(hunk.lines()) {
                if step % CANCEL_STRIDE == 0 && cancelled() {
                    return None;
                }
                builder.push(line.kind());
            }
            builder.flush();
        }
        let widest = widest(&text);
        Some(Self {
            text,
            rows: builder.rows,
            hunks: builder.hunks,
            widest,
        })
    }

    /// Returns the rows in display order.
    #[must_use]
    pub fn rows(&self) -> &[DiffRow] {
        &self.rows
    }

    /// Returns the hunks in display order.
    #[must_use]
    pub fn hunks(&self) -> &[DiffHunk] {
        &self.hunks
    }

    /// Returns whether the lines of both sides differ.
    #[must_use]
    pub const fn has_differences(&self) -> bool {
        self.text.has_differences()
    }

    /// Returns the zero-based `line` of `side` with its terminator, empty past the end.
    #[must_use]
    pub fn line(&self, side: ComparisonSide, line: u32) -> &[u8] {
        let index = line as usize;
        match side {
            ComparisonSide::Old => self.text.old_line(index),
            ComparisonSide::New => self.text.new_line(index),
        }
    }

    /// Returns the number of lines of the longer side.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        count(self.text.old_line_count().max(self.text.new_line_count()))
    }

    /// Returns the longest line of both sides by its characters, a tab counting as four.
    #[must_use]
    pub const fn widest(&self) -> (ComparisonSide, u32) {
        self.widest
    }
}

#[derive(Default)]
struct Builder {
    rows: Vec<DiffRow>,
    hunks: Vec<DiffHunk>,
    at: (u32, u32),
    removed: Vec<u32>,
    added: Vec<u32>,
}

impl Builder {
    fn push(&mut self, kind: LineKind) {
        let (old, new) = self.at;
        match kind {
            LineKind::Context => {
                self.flush();
                self.rows.push(DiffRow::Lines {
                    old: Some(DiffCell { line: old, kind }),
                    new: Some(DiffCell { line: new, kind }),
                });
                self.at = (old.saturating_add(1), new.saturating_add(1));
            }
            LineKind::Removed => {
                if !self.added.is_empty() {
                    self.flush();
                }
                self.removed.push(old);
                self.at.0 = old.saturating_add(1);
            }
            LineKind::Added => {
                self.added.push(new);
                self.at.1 = new.saturating_add(1);
            }
        }
    }

    fn flush(&mut self) {
        let rows = self.removed.len().max(self.added.len());
        let cell = |lines: &[u32], index: usize, kind| {
            lines.get(index).map(|&line| DiffCell { line, kind })
        };
        for index in 0..rows {
            self.rows.push(DiffRow::Lines {
                old: cell(&self.removed, index, LineKind::Removed),
                new: cell(&self.added, index, LineKind::Added),
            });
        }
        self.removed.clear();
        self.added.clear();
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn span(lines: Range<usize>) -> Range<u32> {
    count(lines.start)..count(lines.end)
}

fn widest(text: &TextDiff) -> (ComparisonSide, u32) {
    let longest = |len: usize, line: fn(&TextDiff, usize) -> &[u8]| {
        (0..len)
            .map(|index| (shown_len(line(text, index)), index))
            .max_by_key(|&(bytes, index)| (bytes, std::cmp::Reverse(index)))
            .unwrap_or_default()
    };
    let old = longest(text.old_line_count(), TextDiff::old_line);
    let new = longest(text.new_line_count(), TextDiff::new_line);
    if new.0 > old.0 {
        (ComparisonSide::New, count(new.1))
    } else {
        (ComparisonSide::Old, count(old.1))
    }
}

/// Returns the characters of `line` as shown: UTF-8 sequences count once, a tab four times.
pub(crate) fn shown_len(line: &[u8]) -> usize {
    line.iter()
        .map(|&byte| match byte {
            b'\t' => 4,
            _ if byte & 0xC0 == 0x80 => 0,
            _ => 1,
        })
        .sum()
}
