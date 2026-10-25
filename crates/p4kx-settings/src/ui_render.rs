// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::{Deserialize, Serialize};

/// The size of the pictures the UI tab draws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiRenderSize {
    /// 1024 pixels wide.
    Wide1024,
    /// 2048 pixels wide.
    Wide2048,
    /// 1920 × 1080 pixels.
    FullHd,
    /// One pixel per pixel of the stage or the canvas; also what a value this version does not
    /// know reads as.
    #[default]
    #[serde(other)]
    Stage,
}

/// What lies under the pictures the UI tab draws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiRenderBackground {
    /// The background colour the movie sets.
    Movie,
    /// Opaque black.
    Black,
    /// Opaque white.
    White,
    /// Nothing; also what a value this version does not know reads as.
    #[default]
    #[serde(other)]
    Transparent,
}

/// The file format the UI tab exports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiRenderFormat {
    /// An SVG document.
    Svg,
    /// An 8-bit RGBA PNG; also what a value this version does not know reads as.
    #[default]
    #[serde(other)]
    Png,
}

/// The choices of the UI tab, remembered between sessions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiRenderSettings {
    /// The size of the picture.
    pub size: UiRenderSize,
    /// What lies under it.
    pub background: UiRenderBackground,
    /// The format an export writes.
    pub format: UiRenderFormat,
}
