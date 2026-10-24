// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::DataForgeError;
use p4kx_gfx::GfxError;
use thiserror::Error;

/// A failure to find, read or render a screen.
#[derive(Debug, Error)]
pub enum ScreenError {
    /// No canvas Record has the given GUID or record path.
    #[error("no BuildingBlocks canvas Record matches `{text}`")]
    UnknownCanvas {
        /// The GUID or record path asked for.
        text: String,
    },
    /// Several canvas Records share the given record path.
    #[error("{count} canvas Records share the record path `{text}`; use the GUID of one")]
    AmbiguousCanvas {
        /// The record path asked for.
        text: String,
        /// How many canvas Records have it.
        count: usize,
    },
    /// The Record is no canvas.
    #[error("the Record `{path}` is a {type_name}, not a BuildingBlocks_Canvas")]
    NotACanvas {
        /// The record path.
        path: String,
        /// The type of the Record.
        type_name: String,
    },
    /// The size of the canvas is not a positive size.
    #[error("the canvas `{path}` has no area to draw")]
    EmptyCanvas {
        /// The record path.
        path: String,
    },
    /// A side of the canvas is more than 16384 pixels.
    #[error("the canvas `{path}` is {width} x {height} pixels, more than 16384 a side")]
    CanvasTooLarge {
        /// The record path.
        path: String,
        /// The width in pixels.
        width: f32,
        /// The height in pixels.
        height: f32,
    },
    /// The DataForge could not be read.
    #[error(transparent)]
    DataForge(#[from] DataForgeError),
    /// The scene could not be rasterised or written.
    #[error(transparent)]
    Gfx(#[from] GfxError),
}
