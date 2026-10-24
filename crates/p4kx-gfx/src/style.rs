// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::Serialize;

use crate::GfxError;
use crate::geometry::{Matrix, Rgba};
use crate::reader::TagReader;

/// How a shape's area is filled.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FillStyle {
    /// One colour.
    Solid {
        /// The colour.
        color: Rgba,
    },
    /// A gradient along the x axis of its square from −16384 to 16384 twips.
    LinearGradient {
        /// Maps the gradient square onto the shape.
        matrix: Matrix,
        /// The colours.
        gradient: Gradient,
    },
    /// A gradient from the centre of its circle of radius 16384 twips.
    RadialGradient {
        /// Maps the gradient circle onto the shape.
        matrix: Matrix,
        /// The colours.
        gradient: Gradient,
    },
    /// A radial gradient whose focal point is moved along the x axis.
    FocalGradient {
        /// Maps the gradient circle onto the shape.
        matrix: Matrix,
        /// The colours.
        gradient: Gradient,
        /// The focal point from −1 (left edge) to 1 (right edge).
        focal_point: f32,
    },
    /// A bitmap of the movie, one twip per pixel before the matrix.
    Bitmap {
        /// The ID of the bitmap; `65535` means none.
        bitmap_id: u16,
        /// Maps the bitmap onto the shape.
        matrix: Matrix,
        /// Whether the bitmap repeats beyond its edges.
        repeating: bool,
        /// Whether the bitmap is smoothed when scaled.
        smoothed: bool,
    },
}

/// The colours of a gradient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gradient {
    /// How the area beyond the gradient is filled: 0 pad, 1 reflect, 2 repeat.
    pub spread: u8,
    /// How colours are interpolated: 0 sRGB, 1 linear RGB.
    pub interpolation: u8,
    /// The colour stops in ascending ratio order.
    pub stops: Vec<GradientStop>,
}

/// One colour stop of a gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GradientStop {
    /// The position from 0 to 255.
    pub ratio: u8,
    /// The colour.
    pub color: Rgba,
}

/// How a shape's edges are stroked.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LineStyle {
    /// The width in twips.
    pub width: u16,
    /// The colour; for a line filled with `fill` its first colour.
    pub color: Rgba,
    /// The cap at the start: 0 round, 1 none, 2 square.
    pub start_cap: u8,
    /// The cap at the end: 0 round, 1 none, 2 square.
    pub end_cap: u8,
    /// The join: 0 round, 1 bevel, 2 miter.
    pub join: u8,
    /// The miter limit of a miter join.
    pub miter_limit: Option<f32>,
    /// Whether an open path stays open instead of closing.
    pub no_close: bool,
    /// The fill of the line instead of `color`.
    pub fill: Option<FillStyle>,
}

impl LineStyle {
    /// A line of `width` twips in one colour with round caps and joins.
    pub(crate) const fn plain(width: u16, color: Rgba) -> Self {
        Self {
            width,
            color,
            start_cap: 0,
            end_cap: 0,
            join: 0,
            miter_limit: None,
            no_close: false,
            fill: None,
        }
    }
}

/// The `DefineShape` version a style is read for, 1 to 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShapeVersion(pub(crate) u8);

impl ShapeVersion {
    pub(crate) const fn has_alpha(self) -> bool {
        self.0 >= 3
    }

    pub(crate) const fn extended_counts(self) -> bool {
        self.0 >= 2
    }
}

/// Reads a FILLSTYLEARRAY.
pub(crate) fn read_fill_styles(
    reader: &mut TagReader<'_>,
    version: ShapeVersion,
) -> Result<Vec<FillStyle>, GfxError> {
    let count = reader.count(version.extended_counts())?;
    let mut styles = Vec::with_capacity(count.min(reader.remaining()));
    for _ in 0..count {
        styles.push(read_fill_style(reader, version)?);
    }
    Ok(styles)
}

/// Reads one FILLSTYLE.
pub(crate) fn read_fill_style(
    reader: &mut TagReader<'_>,
    version: ShapeVersion,
) -> Result<FillStyle, GfxError> {
    let kind = reader.u8()?;
    Ok(match kind {
        0x00 => FillStyle::Solid {
            color: reader.color(version.has_alpha())?,
        },
        0x10 => FillStyle::LinearGradient {
            matrix: reader.matrix()?,
            gradient: read_gradient(reader, version)?,
        },
        0x12 => FillStyle::RadialGradient {
            matrix: reader.matrix()?,
            gradient: read_gradient(reader, version)?,
        },
        0x13 => FillStyle::FocalGradient {
            matrix: reader.matrix()?,
            gradient: read_gradient(reader, version)?,
            focal_point: reader.fixed8()?,
        },
        0x40..=0x43 => FillStyle::Bitmap {
            bitmap_id: reader.u16()?,
            matrix: reader.matrix()?,
            repeating: kind & 0x01 == 0,
            smoothed: kind & 0x02 == 0,
        },
        kind => return Err(GfxError::FillStyle { kind }),
    })
}

fn read_gradient(reader: &mut TagReader<'_>, version: ShapeVersion) -> Result<Gradient, GfxError> {
    let spread = u8::try_from(reader.ub(2)?).unwrap_or_default();
    let interpolation = u8::try_from(reader.ub(2)?).unwrap_or_default();
    let count = reader.ub(4)?;
    let mut stops = Vec::with_capacity(count as usize);
    for _ in 0..count {
        stops.push(GradientStop {
            ratio: reader.u8()?,
            color: reader.color(version.has_alpha())?,
        });
    }
    Ok(Gradient {
        spread,
        interpolation,
        stops,
    })
}
