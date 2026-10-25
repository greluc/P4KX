// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the smallest window, taken from the `sidebar` and `workSurface` entries of
//! `docs/design/p4kx-desktop-spec.json`.

use super::desktop::{BUTTON_XS_HEIGHT, SIDEBAR_WIDTH};
use super::tokens::SPACE_1;

/// The width the room of a work tab below the tab bar keeps; a narrower window scrolls it.
pub const WORK_MIN_WIDTH: f32 = 720.0;
/// The height the room of a work tab below the tab bar keeps; a lower window scrolls it.
pub const WORK_MIN_HEIGHT: f32 = 480.0;
/// The window width below which the sidebar hides behind its rail until the rail button shows it:
/// the sidebar and the smallest room of a work tab no longer fit side by side.
pub const SIDEBAR_COLLAPSE_WIDTH: f32 = SIDEBAR_WIDTH + WORK_MIN_WIDTH;
/// The width of the rail that stays of a collapsed sidebar: a compact button between two
/// `space-1` margins.
pub const SIDEBAR_RAIL_WIDTH: f32 = BUTTON_XS_HEIGHT + 2.0 * SPACE_1;
