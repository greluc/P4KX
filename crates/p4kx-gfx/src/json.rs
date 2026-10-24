// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use serde::Serialize;

use crate::GfxError;
use crate::morph::MorphShape;
use crate::shape::Shape;

fn write_json<W: Write>(value: &impl Serialize, mut out: W) -> Result<(), GfxError> {
    serde_json::to_writer_pretty(&mut out, value)
        .map_err(io::Error::from)
        .and_then(|()| out.write_all(b"\n"))
        .map_err(GfxError::Write)
}

impl Shape {
    /// Writes the shape to `out` as pretty-printed JSON: bounds, styles and records in twips.
    ///
    /// # Errors
    ///
    /// Returns [`GfxError::Write`] when `out` fails.
    pub fn write_json<W: Write>(&self, out: W) -> Result<(), GfxError> {
        write_json(self, out)
    }
}

impl MorphShape {
    /// Writes the morph shape to `out` as pretty-printed JSON: both states in twips.
    ///
    /// # Errors
    ///
    /// Returns [`GfxError::Write`] when `out` fails.
    pub fn write_json<W: Write>(&self, out: W) -> Result<(), GfxError> {
        write_json(self, out)
    }
}
