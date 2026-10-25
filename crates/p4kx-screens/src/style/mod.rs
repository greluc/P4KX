// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod entry;
mod geometry;
mod matching;
mod sheet;
mod text;

pub(crate) use entry::{Entry, Modifiers, inline_entries, modifiers, parse_entries};
pub(crate) use geometry::GeometryModifiers;
pub(crate) use matching::{Subject, TagPaths};
pub(crate) use sheet::{CanvasStyles, StyleCache, merge, resolve};
pub(crate) use text::TextModifiers;
