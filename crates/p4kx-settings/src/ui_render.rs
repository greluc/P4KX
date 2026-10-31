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

/// The game state the UI tab draws screens in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiRenderPower {
    /// The ship is powered on.
    PoweredOn,
    /// Every game variable at zero; also what a value this version does not know reads as.
    #[default]
    #[serde(other)]
    AtRest,
}

/// The most bytes of typed game variables that are kept.
pub const MAX_VARIABLES_BYTES: usize = 4096;

/// The choices of the UI tab, remembered between sessions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiRenderSettings {
    /// The size of the picture.
    pub size: UiRenderSize,
    /// What lies under it.
    pub background: UiRenderBackground,
    /// The format an export writes.
    pub format: UiRenderFormat,
    /// The game state a screen is drawn in.
    pub power: UiRenderPower,
    /// The game variables typed beside the state, as `NAME=VALUE` settings separated by blanks or
    /// commas. When it is read, a text longer than [`MAX_VARIABLES_BYTES`] bytes is cut with
    /// [`bound_variables`]: at the last separator before the limit, empty without one.
    #[serde(deserialize_with = "bounded")]
    pub variables: String,
}

const fn is_separator(c: char) -> bool {
    c.is_whitespace() || c == ','
}

/// Returns `text` held to [`MAX_VARIABLES_BYTES`] bytes: a longer text is cut at the last blank or
/// comma before the limit, so that no setting is cut in two, and is empty when it has none.
#[must_use]
pub fn bound_variables(text: &str) -> &str {
    if text.len() <= MAX_VARIABLES_BYTES {
        return text;
    }
    let head = (0..=MAX_VARIABLES_BYTES)
        .rev()
        .find(|&at| text.is_char_boundary(at))
        .unwrap_or(0);
    if text[head..].starts_with(is_separator) {
        return &text[..head];
    }
    text[..head]
        .rfind(is_separator)
        .map_or("", |at| &text[..at])
}

fn bounded<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let text = String::deserialize(deserializer)?;
    Ok(bound_variables(&text).to_owned())
}
