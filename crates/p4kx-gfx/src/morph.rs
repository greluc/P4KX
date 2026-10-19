use serde::Serialize;

use crate::GfxError;
use crate::geometry::{Matrix, Rect};
use crate::line::{LineFlags, fill_color};
use crate::reader::TagReader;
use crate::shape::{RecordBudget, ShapeRecord, read_records};
use crate::style::{FillStyle, Gradient, GradientStop, LineStyle, ShapeVersion};
use crate::tag::Tag;

const STYLE_VERSION: ShapeVersion = ShapeVersion(3);

/// A shape of a `DefineMorphShape` tag that morphs from a start to an end state, in twips.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MorphShape {
    /// The ID of the morph shape.
    pub id: u16,
    /// The `DefineMorphShape` version, 1 or 2.
    pub version: u8,
    /// The bounds of the start state.
    pub start_bounds: Rect,
    /// The bounds of the end state.
    pub end_bounds: Rect,
    /// The fill styles of the start state, numbered from 1.
    pub start_fill_styles: Vec<FillStyle>,
    /// The fill styles of the end state, paired with the start fill styles.
    pub end_fill_styles: Vec<FillStyle>,
    /// The line styles of the start state, numbered from 1.
    pub start_line_styles: Vec<LineStyle>,
    /// The line styles of the end state, paired with the start line styles.
    pub end_line_styles: Vec<LineStyle>,
    /// The records of the start state.
    pub start_records: Vec<ShapeRecord>,
    /// The edges of the end state, paired with the edges of the start state.
    pub end_records: Vec<ShapeRecord>,
}

/// Reads a `DefineMorphShape` or `DefineMorphShape2` tag.
pub(crate) fn read_morph_shape(
    tag: &Tag<'_>,
    version: u8,
    budget: &mut RecordBudget,
) -> Result<MorphShape, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let start_bounds = reader.rect()?;
    let end_bounds = reader.rect()?;
    if version >= 2 {
        reader.rect()?;
        reader.rect()?;
        reader.u8()?;
    }
    let end_edges_offset = reader.u32()?;
    let end_edges_start = (reader.offset() - tag.offset)
        .checked_add(usize::try_from(end_edges_offset).unwrap_or(usize::MAX));
    let (start_fill_styles, end_fill_styles) = read_pairs(&mut reader, read_fill)?;
    let (start_line_styles, end_line_styles) =
        read_pairs(&mut reader, |reader| read_line(reader, version))?;
    let start_records = read_records(&mut reader, STYLE_VERSION, budget)?;
    let end_start = end_edges_start
        .filter(|&start| start <= tag.body.len())
        .ok_or(GfxError::MorphEndEdges {
            offset: end_edges_offset,
        })?;
    let mut end_reader = TagReader::new(
        tag.body.get(end_start..).unwrap_or_default(),
        tag.offset + end_start,
    );
    let end_records = read_records(&mut end_reader, STYLE_VERSION, budget)?;
    Ok(MorphShape {
        id,
        version,
        start_bounds,
        end_bounds,
        start_fill_styles,
        end_fill_styles,
        start_line_styles,
        end_line_styles,
        start_records,
        end_records,
    })
}

type Pair<T> = (T, T);

fn read_pairs<T>(
    reader: &mut TagReader<'_>,
    mut read: impl FnMut(&mut TagReader<'_>) -> Result<Pair<T>, GfxError>,
) -> Result<Pair<Vec<T>>, GfxError> {
    let count = reader.count(true)?;
    let capacity = count.min(reader.remaining());
    let mut start = Vec::with_capacity(capacity);
    let mut end = Vec::with_capacity(capacity);
    for _ in 0..count {
        let (start_style, end_style) = read(reader)?;
        start.push(start_style);
        end.push(end_style);
    }
    Ok((start, end))
}

fn read_fill(reader: &mut TagReader<'_>) -> Result<Pair<FillStyle>, GfxError> {
    let kind = reader.u8()?;
    Ok(match kind {
        0x00 => (
            FillStyle::Solid {
                color: reader.rgba()?,
            },
            FillStyle::Solid {
                color: reader.rgba()?,
            },
        ),
        0x10 | 0x12 | 0x13 => {
            let start_matrix = reader.matrix()?;
            let end_matrix = reader.matrix()?;
            let (start_gradient, end_gradient) = read_gradients(reader)?;
            let gradient = |matrix: Matrix, gradient: Gradient, focal_point| match kind {
                0x10 => FillStyle::LinearGradient { matrix, gradient },
                0x12 => FillStyle::RadialGradient { matrix, gradient },
                _ => FillStyle::FocalGradient {
                    matrix,
                    gradient,
                    focal_point,
                },
            };
            let (start_focal, end_focal) = if kind == 0x13 {
                (reader.fixed8()?, reader.fixed8()?)
            } else {
                (0.0, 0.0)
            };
            (
                gradient(start_matrix, start_gradient, start_focal),
                gradient(end_matrix, end_gradient, end_focal),
            )
        }
        0x40..=0x43 => {
            let bitmap_id = reader.u16()?;
            let bitmap = |matrix| FillStyle::Bitmap {
                bitmap_id,
                matrix,
                repeating: kind & 0x01 == 0,
                smoothed: kind & 0x02 == 0,
            };
            (bitmap(reader.matrix()?), bitmap(reader.matrix()?))
        }
        kind => return Err(GfxError::FillStyle { kind }),
    })
}

fn read_gradients(reader: &mut TagReader<'_>) -> Result<Pair<Gradient>, GfxError> {
    let spread = u8::try_from(reader.ub(2)?).unwrap_or_default();
    let interpolation = u8::try_from(reader.ub(2)?).unwrap_or_default();
    let count = reader.ub(4)? as usize;
    let mut start = Vec::with_capacity(count);
    let mut end = Vec::with_capacity(count);
    for _ in 0..count {
        start.push(GradientStop {
            ratio: reader.u8()?,
            color: reader.rgba()?,
        });
        end.push(GradientStop {
            ratio: reader.u8()?,
            color: reader.rgba()?,
        });
    }
    let gradient = |stops| Gradient {
        spread,
        interpolation,
        stops,
    };
    Ok((gradient(start), gradient(end)))
}

fn read_line(reader: &mut TagReader<'_>, version: u8) -> Result<Pair<LineStyle>, GfxError> {
    let start_width = reader.u16()?;
    let end_width = reader.u16()?;
    if version < 2 {
        return Ok((
            LineStyle::plain(start_width, reader.rgba()?),
            LineStyle::plain(end_width, reader.rgba()?),
        ));
    }
    let flags = LineFlags::read(reader)?;
    if flags.has_fill {
        let (start_fill, end_fill) = read_fill(reader)?;
        Ok((
            flags.style(start_width, fill_color(&start_fill), Some(start_fill)),
            flags.style(end_width, fill_color(&end_fill), Some(end_fill)),
        ))
    } else {
        Ok((
            flags.style(start_width, reader.rgba()?, None),
            flags.style(end_width, reader.rgba()?, None),
        ))
    }
}
