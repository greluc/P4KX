// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the UI tab, taken from the `uiTab` entry of
//! `docs/design/p4kx-desktop-spec.json`.

use super::desktop::{ROW_FONT, ROW_HEIGHT};

/// The height of a row of the lists of the UI tab, which stack a name over a detail line: a list
/// row and one more line of the row font.
pub const UI_ROW_HEIGHT: f32 = ROW_HEIGHT + ROW_FONT;
/// The most a picture that is smaller than the room is scaled up to fill it.
pub const UI_PICTURE_MAX_SCALE: f32 = 4.0;
/// The lines of the report of a render that are laid out; the rest is counted.
pub const UI_REPORT_MAX_LINES: usize = 2_000;
