// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// The bounds of one composition; the Records are untrusted, so work past a bound is not done and
/// [`Report::truncated`](crate::Report::truncated) says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most widgets visited or placed, hidden ones and those of nested canvases included.
    pub widgets: usize,
    /// The most steps of the style cascade: entries tried and selectors evaluated, summed over
    /// the whole screen.
    pub evaluations: u64,
    /// The deepest nesting of widgets and canvases.
    pub depth: usize,
    /// The most templates clones and lists instantiate, summed over the whole screen.
    pub instances: usize,
    /// The most items one list repeats its template for.
    pub rows: usize,
}

impl Limits {
    /// The bounds `p4kx` uses: 2^18 widgets, 2^26 cascade steps, a depth of 64, 2^14 instances
    /// and 2^10 rows per list.
    pub const DEFAULT: Self = Self {
        widgets: 1 << 18,
        evaluations: 1 << 26,
        depth: 64,
        instances: 1 << 14,
        rows: 1 << 10,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}
