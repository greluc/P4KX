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
}

impl Limits {
    /// The bounds `p4kx` uses: 2^18 widgets, 2^26 cascade steps and a depth of 64.
    pub const DEFAULT: Self = Self {
        widgets: 1 << 18,
        evaluations: 1 << 26,
        depth: 64,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}
