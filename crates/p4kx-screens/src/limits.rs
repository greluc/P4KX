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
    /// The most pixels all textures of the screen have together, and the most one may have.
    pub texture_pixels: u64,
    /// The most path commands all vector graphics of the screen have together.
    pub vector_commands: usize,
}

impl Limits {
    /// The bounds `p4kx` uses: 2^18 widgets, 2^26 cascade steps, a depth of 64, 2^26 texture
    /// pixels (2^25 for one texture) and 2^22 path commands.
    pub const DEFAULT: Self = Self {
        widgets: 1 << 18,
        evaluations: 1 << 26,
        depth: 64,
        texture_pixels: 1 << 26,
        vector_commands: 1 << 22,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}
