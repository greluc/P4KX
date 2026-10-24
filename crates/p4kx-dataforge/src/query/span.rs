// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ops::Range;

/// A range of bytes in the text of a Query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// Creates the span from `start` up to, but not including, `end`; an `end` before `start` is
    /// taken as `start`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end: if end < start { start } else { end },
        }
    }

    /// Returns the offset of the first byte.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the offset behind the last byte.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Returns the byte range.
    #[must_use]
    pub const fn range(self) -> Range<usize> {
        self.start..self.end
    }

    pub(crate) const fn to(self, other: Self) -> Self {
        Self::new(self.start, other.end)
    }
}
