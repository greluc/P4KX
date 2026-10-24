// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod entry;
mod matching;
mod sheet;

pub(crate) use entry::{Entry, Modifiers, inline_entries, parse_entries};
pub(crate) use matching::{Subject, TagPaths};
pub(crate) use sheet::{CanvasStyles, StyleCache, resolve};
