// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::GfxError;
use crate::geometry::Rgba;
use crate::reader::TagReader;
use crate::style::{FillStyle, LineStyle, ShapeVersion, read_fill_style};

/// The caps, join and flags of a LINESTYLE2 or MORPHLINESTYLE2, read after its widths.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LineFlags {
    pub(crate) start_cap: u8,
    pub(crate) end_cap: u8,
    pub(crate) join: u8,
    pub(crate) has_fill: bool,
    pub(crate) no_close: bool,
    pub(crate) miter_limit: Option<f32>,
}

impl LineFlags {
    /// Reads the flags and the miter limit of a miter join.
    pub(crate) fn read(reader: &mut TagReader<'_>) -> Result<Self, GfxError> {
        let start_cap = two_bits(reader)?;
        let join = two_bits(reader)?;
        let has_fill = reader.flag()?;
        reader.ub(3)?;
        reader.ub(5)?;
        let no_close = reader.flag()?;
        let end_cap = two_bits(reader)?;
        let miter_limit = if join == 2 {
            Some(reader.fixed8()?)
        } else {
            None
        };
        Ok(Self {
            start_cap,
            end_cap,
            join,
            has_fill,
            no_close,
            miter_limit,
        })
    }

    /// Builds the line style of `width` twips stroked with `color` or filled with `fill`.
    pub(crate) const fn style(self, width: u16, color: Rgba, fill: Option<FillStyle>) -> LineStyle {
        LineStyle {
            width,
            color,
            start_cap: self.start_cap,
            end_cap: self.end_cap,
            join: self.join,
            miter_limit: self.miter_limit,
            no_close: self.no_close,
            fill,
        }
    }
}

/// Returns the colour that stands for a fill: its own for a solid fill, otherwise black.
pub(crate) const fn fill_color(fill: &FillStyle) -> Rgba {
    match fill {
        FillStyle::Solid { color } => *color,
        _ => Rgba {
            red: 0,
            green: 0,
            blue: 0,
            alpha: u8::MAX,
        },
    }
}

/// Reads a LINESTYLEARRAY of LINESTYLE, or of LINESTYLE2 for version 4.
pub(crate) fn read_line_styles(
    reader: &mut TagReader<'_>,
    version: ShapeVersion,
) -> Result<Vec<LineStyle>, GfxError> {
    let count = reader.count(version.extended_counts())?;
    let mut styles = Vec::with_capacity(count.min(reader.remaining()));
    for _ in 0..count {
        let width = reader.u16()?;
        let style = if version.0 >= 4 {
            let flags = LineFlags::read(reader)?;
            if flags.has_fill {
                let fill = read_fill_style(reader, version)?;
                flags.style(width, fill_color(&fill), Some(fill))
            } else {
                flags.style(width, reader.rgba()?, None)
            }
        } else {
            LineStyle::plain(width, reader.color(version.has_alpha())?)
        };
        styles.push(style);
    }
    Ok(styles)
}

fn two_bits(reader: &mut TagReader<'_>) -> Result<u8, GfxError> {
    Ok(u8::try_from(reader.ub(2)?).unwrap_or_default())
}
