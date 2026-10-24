// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the brand in the header, taken from the `header` entry of
//! `docs/design/p4kx-desktop-spec.json`; they are re-exported by [`super::desktop`].

use super::tokens::{FS_SM, SPACE_2, SPACE_8};

/// The side of the logo mark in the header.
pub const BRAND_MARK: f32 = SPACE_8;
/// The gap between the logo mark and the wordmark.
pub const BRAND_GAP: f32 = SPACE_2;
/// The height of the wordmark row in the header.
pub const BRAND_HEIGHT: f32 = 24.0;
/// The gap between the wordmark and the menus.
pub const BRAND_PADDING_RIGHT: f32 = 24.0;
/// The font size of the wordmark.
pub const WORDMARK_SIZE: f32 = FS_SM;
