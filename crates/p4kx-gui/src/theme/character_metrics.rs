// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the Characters tab, taken from the `characterBuilder` entry of
//! `docs/design/p4kx-desktop-spec.json`.

use super::desktop::ROW_HEIGHT;

/// The width of the work surface below which the choices, the 3D view and the report stack in
/// one column that scrolls, instead of standing side by side.
pub const CHARACTER_STACK_WIDTH: f32 = 560.0;
/// The rows of the report that stay visible below the 3D view.
pub const CHARACTER_REPORT_ROWS: f32 = 3.0;
/// The height of the report below the 3D view: its head row and [`CHARACTER_REPORT_ROWS`] rows.
pub const CHARACTER_REPORT_HEIGHT: f32 = ROW_HEIGHT * (1.0 + CHARACTER_REPORT_ROWS);
/// The rows of the list of expressions that stay visible before it scrolls.
pub const CHARACTER_EXPRESSION_ROWS: f32 = 5.0;
