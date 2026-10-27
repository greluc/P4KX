// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the smallest window, taken from the `sidebar` and `workSurface` entries of
//! `docs/design/p4kx-desktop-spec.json`.

use super::desktop::{
    BUTTON_HEIGHT, BUTTON_XS_HEIGHT, ROW_HEIGHT, SEGMENT_HEIGHT, SIDEBAR_WIDTH, TOOLBAR_PADDING,
};
use super::tokens::SPACE_1;

/// The width the room of a work tab below the tab bar keeps; a narrower window scrolls it.
///
/// The smallest window of 640 points less the rail and the scrollbar a low window shows.
pub const WORK_MIN_WIDTH: f32 = 580.0;
/// The height the room of a work tab below the tab bar keeps; a lower window scrolls it.
pub const WORK_MIN_HEIGHT: f32 = 536.0;
/// The window width below which the sidebar hides behind its rail until the rail button shows it:
/// the sidebar and the smallest room of a work tab no longer fit side by side.
pub const SIDEBAR_COLLAPSE_WIDTH: f32 = SIDEBAR_WIDTH + WORK_MIN_WIDTH;
/// The width of the rail that stays of a collapsed sidebar: a compact button between two
/// `space-1` margins.
pub const SIDEBAR_RAIL_WIDTH: f32 = BUTTON_XS_HEIGHT + 2.0 * SPACE_1;
/// The height the Entry list of the Browse tab keeps above the preview.
///
/// Its toolbar with the search row, the actions in up to two rows and the quick filters, the table
/// head and one row, and the two hairlines of its frame.
pub const BROWSE_LIST_MIN_HEIGHT: f32 =
    5.0 * TOOLBAR_PADDING + 3.0 * BUTTON_HEIGHT + SEGMENT_HEIGHT + 2.0 * ROW_HEIGHT + 2.0;
/// The height the toolbar of the Sounds tab keeps before it scrolls.
///
/// The Entity row, two rows of actions and the search row.
pub const SOUNDS_TOOLBAR_MIN_HEIGHT: f32 = 4.0 * BUTTON_HEIGHT + 3.0 * TOOLBAR_PADDING;
