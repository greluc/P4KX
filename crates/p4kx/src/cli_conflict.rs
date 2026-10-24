// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use clap::ValueEnum;

/// The values of `p4kx extract --conflict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Conflict {
    /// Keep the existing file.
    Skip,
    /// Replace the existing file.
    Overwrite,
    /// Replace the existing file only when its size or CRC-32C differs from the new content.
    IfChanged,
}
