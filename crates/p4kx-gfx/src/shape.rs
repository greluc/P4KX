use serde::Serialize;

use crate::GfxError;
use crate::geometry::{Point, Rect};
use crate::line::read_line_styles;
use crate::reader::TagReader;
use crate::style::{FillStyle, LineStyle, ShapeVersion, read_fill_styles};
use crate::tag::Tag;

/// A vector shape of a `DefineShape` tag, in twips as the file stores it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shape {
    /// The ID of the shape.
    pub id: u16,
    /// The `DefineShape` version, 1 to 4.
    pub version: u8,
    /// The bounds including the strokes.
    pub bounds: Rect,
    /// The bounds of the edges without strokes, from version 4 on.
    pub edge_bounds: Option<Rect>,
    /// The fill styles the first style records refer to, numbered from 1.
    pub fill_styles: Vec<FillStyle>,
    /// The line styles the first style records refer to, numbered from 1.
    pub line_styles: Vec<LineStyle>,
    /// The edges and style changes in drawing order.
    pub records: Vec<ShapeRecord>,
}

/// One record of a shape: a style change or an edge relative to the current position.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShapeRecord {
    /// Moves the pen or changes the styles.
    StyleChange(Box<StyleChange>),
    /// A straight edge.
    StraightEdge {
        /// The movement to the end point.
        delta: Point,
    },
    /// A quadratic Bézier edge.
    CurvedEdge {
        /// The movement to the control point.
        control: Point,
        /// The movement from the control point to the end point.
        anchor: Point,
    },
}

/// A record that moves the pen or changes the styles of the following edges.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StyleChange {
    /// The new absolute pen position.
    pub move_to: Option<Point>,
    /// The new fill on the left of the edges, 0 for none.
    pub fill_style0: Option<u32>,
    /// The new fill on the right of the edges, 0 for none.
    pub fill_style1: Option<u32>,
    /// The new line style, 0 for none.
    pub line_style: Option<u32>,
    /// New style arrays that the style numbers of this and the later records refer to.
    pub new_styles: Option<Box<NewStyles>>,
}

/// Style arrays that replace the current ones within a shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NewStyles {
    /// The fill styles, numbered from 1.
    pub fill_styles: Vec<FillStyle>,
    /// The line styles, numbered from 1.
    pub line_styles: Vec<LineStyle>,
}

/// The most shape records of all shapes, morph shapes and glyphs a movie may hold, far above the
/// records of the largest movie of a real Archive.
pub const MAX_SHAPE_RECORDS: usize = 1 << 24;

/// The shape records a movie may still hold.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecordBudget(usize);

impl RecordBudget {
    /// The budget of a whole movie.
    pub(crate) const MOVIE: Self = Self(MAX_SHAPE_RECORDS);

    /// Takes one record from the budget.
    pub(crate) fn take(&mut self) -> Result<(), GfxError> {
        self.0 = self.0.checked_sub(1).ok_or(GfxError::TooManyRecords {
            limit: MAX_SHAPE_RECORDS,
        })?;
        Ok(())
    }
}

/// Reads a `DefineShape`, `DefineShape2`, `DefineShape3` or `DefineShape4` tag.
pub(crate) fn read_shape(
    tag: &Tag<'_>,
    version: u8,
    budget: &mut RecordBudget,
) -> Result<Shape, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let bounds = reader.rect()?;
    let edge_bounds = if version >= 4 {
        let edge_bounds = reader.rect()?;
        reader.u8()?;
        Some(edge_bounds)
    } else {
        None
    };
    let style_version = ShapeVersion(version);
    let fill_styles = read_fill_styles(&mut reader, style_version)?;
    let line_styles = read_line_styles(&mut reader, style_version)?;
    let records = read_records(&mut reader, style_version, budget)?;
    Ok(Shape {
        id,
        version,
        bounds,
        edge_bounds,
        fill_styles,
        line_styles,
        records,
    })
}

/// Reads the bit widths and the records of a SHAPE up to its end record.
pub(crate) fn read_records(
    reader: &mut TagReader<'_>,
    version: ShapeVersion,
    budget: &mut RecordBudget,
) -> Result<Vec<ShapeRecord>, GfxError> {
    reader.align();
    let mut fill_bits = reader.ub(4)?;
    let mut line_bits = reader.ub(4)?;
    let mut records = Vec::new();
    loop {
        budget.take()?;
        if reader.flag()? {
            records.push(read_edge(reader)?);
            continue;
        }
        let new_styles = reader.flag()?;
        let line_style = reader.flag()?;
        let fill_style1 = reader.flag()?;
        let fill_style0 = reader.flag()?;
        let move_to = reader.flag()?;
        if !(new_styles || line_style || fill_style1 || fill_style0 || move_to) {
            return Ok(records);
        }
        let mut change = StyleChange {
            move_to: None,
            fill_style0: None,
            fill_style1: None,
            line_style: None,
            new_styles: None,
        };
        if move_to {
            let bits = reader.ub(5)?;
            change.move_to = Some(Point {
                x: reader.sb(bits)?,
                y: reader.sb(bits)?,
            });
        }
        if fill_style0 {
            change.fill_style0 = Some(reader.ub(fill_bits)?);
        }
        if fill_style1 {
            change.fill_style1 = Some(reader.ub(fill_bits)?);
        }
        if line_style {
            change.line_style = Some(reader.ub(line_bits)?);
        }
        if new_styles {
            let styles = NewStyles {
                fill_styles: read_fill_styles(reader, version)?,
                line_styles: read_line_styles(reader, version)?,
            };
            fill_bits = reader.ub(4)?;
            line_bits = reader.ub(4)?;
            change.new_styles = Some(Box::new(styles));
        }
        records.push(ShapeRecord::StyleChange(Box::new(change)));
    }
}

fn read_edge(reader: &mut TagReader<'_>) -> Result<ShapeRecord, GfxError> {
    let straight = reader.flag()?;
    let bits = reader.ub(4)? + 2;
    if straight {
        let general = reader.flag()?;
        let delta = if general {
            Point {
                x: reader.sb(bits)?,
                y: reader.sb(bits)?,
            }
        } else if reader.flag()? {
            Point {
                x: 0,
                y: reader.sb(bits)?,
            }
        } else {
            Point {
                x: reader.sb(bits)?,
                y: 0,
            }
        };
        Ok(ShapeRecord::StraightEdge { delta })
    } else {
        Ok(ShapeRecord::CurvedEdge {
            control: Point {
                x: reader.sb(bits)?,
                y: reader.sb(bits)?,
            },
            anchor: Point {
                x: reader.sb(bits)?,
                y: reader.sb(bits)?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{RecordBudget, read_records};
    use crate::GfxError;
    use crate::reader::TagReader;
    use crate::style::ShapeVersion;

    fn bytes(bits: &str) -> Vec<u8> {
        let bits: Vec<u8> = bits
            .bytes()
            .filter(|bit| *bit != b' ')
            .map(|bit| bit - b'0')
            .collect();
        bits.chunks(8)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .fold(0, |byte, (index, bit)| byte | (bit << (7 - index)))
            })
            .collect()
    }

    #[test]
    fn records_beyond_the_movie_budget_are_refused() {
        let edge = "11 0000 0 0 01 ";
        let data = bytes(&format!("0000 0000 {edge}{edge}{edge}000000"));
        let mut reader = TagReader::new(&data, 0);
        let mut budget = RecordBudget(3);
        assert!(
            matches!(
                read_records(&mut reader, ShapeVersion(1), &mut budget),
                Err(GfxError::TooManyRecords { .. })
            ),
            "three edges and the end record need four records"
        );
        let mut reader = TagReader::new(&data, 0);
        let mut budget = RecordBudget(4);
        let records = read_records(&mut reader, ShapeVersion(1), &mut budget);
        assert_eq!(records.map(|records| records.len()).ok(), Some(3), "fits");
    }
}
