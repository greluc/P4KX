// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::gfx::{BitWriter, rect, signed_bits, translation};
use crate::gfx_place::write_matrix;

/// A fill style of a [`TestShape`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestFill {
    /// One colour, RGBA; written as RGB below `DefineShape3`.
    Solid([u8; 4]),
    /// A linear gradient from the first to the second colour.
    Linear([u8; 4], [u8; 4]),
    /// A repeating, smoothed bitmap fill of the bitmap with this ID, moved by (x, y) twips.
    Bitmap(u16, i32, i32),
    /// A gradient of the first colour at ratio 0 and the second at 255, laid out as the [`TestGradient`] says.
    Gradient([u8; 4], [u8; 4], TestGradient),
    /// A bitmap fill of the bitmap `id`, `scale` twips per pixel, its corner at `at`.
    ScaledBitmap {
        /// The ID of the bitmap.
        id: u16,
        /// The twips per bitmap pixel.
        scale: i32,
        /// The position of its top left corner in twips.
        at: (i32, i32),
        /// Whether it repeats.
        repeating: bool,
        /// Whether it is smoothed.
        smoothed: bool,
    },
}

/// How a [`TestFill::Gradient`] lies on its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestGradient {
    /// A linear gradient from x to x + width twips, along the x axis.
    Linear {
        /// The start in twips.
        x: i32,
        /// The length in twips.
        width: i32,
    },
    /// A radial gradient around a centre with a radius in twips; `focal` moves its focal point,
    /// 256 being the right edge, and makes it a focal gradient when not 0.
    Radial {
        /// The centre in twips.
        center: (i32, i32),
        /// The radius in twips.
        radius: i32,
        /// The focal point in 8.8 fixed point.
        focal: i16,
    },
}

/// A record of a [`TestShape`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestShapeRecord {
    /// Moves the pen to (x, y) and selects fill style 0, fill style 1 and the line style.
    Move {
        /// The new pen position.
        to: (i32, i32),
        /// The fill on the left, 0 for none.
        fill0: u32,
        /// The fill on the right, 0 for none.
        fill1: u32,
        /// The line style, 0 for none.
        line: u32,
    },
    /// A straight edge; one written as horizontal or vertical when the other delta is 0.
    Line(i32, i32),
    /// A curved edge to the control point and on to the anchor.
    Curve(i32, i32, i32, i32),
}

/// Writes the body of a `DefineShape` tag of version 1 to 4.
#[derive(Debug, Clone)]
pub struct TestShape {
    id: u16,
    version: u8,
    bounds: [i32; 4],
    fills: Vec<TestFill>,
    lines: Vec<(u16, [u8; 4])>,
    records: Vec<TestShapeRecord>,
}

impl TestShape {
    /// Starts a shape with the given ID, `DefineShape` version and bounds x min, x max, y min,
    /// y max in twips.
    #[must_use]
    pub const fn new(id: u16, version: u8, bounds: [i32; 4]) -> Self {
        Self {
            id,
            version,
            bounds,
            fills: Vec::new(),
            lines: Vec::new(),
            records: Vec::new(),
        }
    }

    /// Adds a fill style.
    #[must_use]
    pub fn fill(mut self, fill: TestFill) -> Self {
        self.fills.push(fill);
        self
    }

    /// Adds a line style of `width` twips in an RGBA colour.
    #[must_use]
    pub fn line(mut self, width: u16, color: [u8; 4]) -> Self {
        self.lines.push((width, color));
        self
    }

    /// Adds a record.
    #[must_use]
    pub fn record(mut self, record: TestShapeRecord) -> Self {
        self.records.push(record);
        self
    }

    /// Returns the tag code of the shape's version.
    #[must_use]
    pub const fn code(&self) -> u16 {
        match self.version {
            1 => 2,
            2 => 22,
            3 => 32,
            _ => 83,
        }
    }

    /// Returns the tag body.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        let mut bits = BitWriter::default();
        bits.bytes(&self.id.to_le_bytes());
        rect(&mut bits, self.bounds);
        if self.version >= 4 {
            rect(&mut bits, self.bounds);
            bits.bytes(&[0]);
        }
        let alpha = self.version >= 3;
        bits.bytes(&[u8::try_from(self.fills.len()).unwrap_or(u8::MAX)]);
        for fill in &self.fills {
            write_fill(&mut bits, *fill, alpha);
        }
        bits.bytes(&[u8::try_from(self.lines.len()).unwrap_or(u8::MAX)]);
        for &(width, color) in &self.lines {
            bits.bytes(&width.to_le_bytes());
            if self.version >= 4 {
                bits.ub(16, 0);
            }
            write_color(&mut bits, color, alpha || self.version >= 4);
        }
        let fill_bits = index_bits(self.fills.len());
        let line_bits = index_bits(self.lines.len());
        write_records(&mut bits, fill_bits, line_bits, &self.records);
        bits.finish()
    }
}

const fn index_bits(count: usize) -> u32 {
    count.bit_width()
}

fn write_color(bits: &mut BitWriter, [red, green, blue, alpha]: [u8; 4], with_alpha: bool) {
    if with_alpha {
        bits.bytes(&[red, green, blue, alpha]);
    } else {
        bits.bytes(&[red, green, blue]);
    }
}

fn write_fill(bits: &mut BitWriter, fill: TestFill, alpha: bool) {
    match fill {
        TestFill::Solid(color) => {
            bits.bytes(&[0x00]);
            write_color(bits, color, alpha);
        }
        TestFill::Linear(start, end) => {
            bits.bytes(&[0x10]);
            translation(bits, 0, 0);
            bits.bytes(&[0x02]);
            bits.bytes(&[0]);
            write_color(bits, start, alpha);
            bits.bytes(&[255]);
            write_color(bits, end, alpha);
        }
        TestFill::Bitmap(id, x, y) => {
            bits.bytes(&[0x40]);
            bits.bytes(&id.to_le_bytes());
            translation(bits, x, y);
        }
        TestFill::Gradient(start, end, layout) => {
            write_gradient(bits, (start, end), layout, alpha);
        }
        TestFill::ScaledBitmap {
            id,
            scale,
            at,
            repeating,
            smoothed,
        } => {
            bits.bytes(&[0x40 | u8::from(!repeating) | u8::from(!smoothed) << 1]);
            bits.bytes(&id.to_le_bytes());
            let factor = ratio(scale, 1);
            write_matrix(bits, [factor, factor, 0.0, 0.0], at);
        }
    }
}

fn ratio(numerator: i32, denominator: i32) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
    let value = numerator as f32 / denominator as f32;
    value
}

fn write_gradient(
    bits: &mut BitWriter,
    (start, end): ([u8; 4], [u8; 4]),
    layout: TestGradient,
    alpha: bool,
) {
    match layout {
        TestGradient::Linear { x, width } => {
            bits.bytes(&[0x10]);
            let scale = ratio(width, 32_768);
            write_matrix(bits, [scale, scale, 0.0, 0.0], (x + width / 2, 0));
        }
        TestGradient::Radial {
            center,
            radius,
            focal,
        } => {
            bits.bytes(&[if focal == 0 { 0x12 } else { 0x13 }]);
            let scale = ratio(radius, 16_384);
            write_matrix(bits, [scale, scale, 0.0, 0.0], center);
        }
    }
    bits.bytes(&[0x02]);
    bits.bytes(&[0]);
    write_color(bits, start, alpha);
    bits.bytes(&[255]);
    write_color(bits, end, alpha);
    if let TestGradient::Radial { focal, .. } = layout
        && focal != 0
    {
        bits.bytes(&focal.to_le_bytes());
    }
}

/// Writes the bit widths, the records and the end record of a SHAPE.
pub(crate) fn write_records(
    bits: &mut BitWriter,
    fill_bits: u32,
    line_bits: u32,
    records: &[TestShapeRecord],
) {
    bits.align();
    bits.ub(4, fill_bits);
    bits.ub(4, line_bits);
    for record in records {
        match *record {
            TestShapeRecord::Move {
                to,
                fill0,
                fill1,
                line,
            } => {
                bits.flag(false);
                bits.ub(5, 0b01111);
                let size = signed_bits(&<[i32; 2]>::from(to));
                bits.ub(5, size);
                bits.sb(size, to.0);
                bits.sb(size, to.1);
                bits.ub(fill_bits, fill0);
                bits.ub(fill_bits, fill1);
                bits.ub(line_bits, line);
            }
            TestShapeRecord::Line(dx, dy) => {
                bits.flag(true);
                bits.flag(true);
                let size = signed_bits(&[dx, dy]).max(2);
                bits.ub(4, size - 2);
                if dx != 0 && dy != 0 {
                    bits.flag(true);
                    bits.sb(size, dx);
                    bits.sb(size, dy);
                } else {
                    bits.flag(false);
                    bits.flag(dx == 0);
                    bits.sb(size, if dx == 0 { dy } else { dx });
                }
            }
            TestShapeRecord::Curve(cx, cy, ax, ay) => {
                bits.flag(true);
                bits.flag(false);
                let size = signed_bits(&[cx, cy, ax, ay]).max(2);
                bits.ub(4, size - 2);
                for value in [cx, cy, ax, ay] {
                    bits.sb(size, value);
                }
            }
        }
    }
    bits.ub(6, 0);
    bits.align();
}

/// Writes the body of a `DefineMorphShape` tag with one solid fill morphing from `start_color`
/// to `end_color` and the given start and end records.
#[must_use]
pub fn morph_shape_body(
    id: u16,
    (start_color, end_color): ([u8; 4], [u8; 4]),
    start: &[TestShapeRecord],
    end: &[TestShapeRecord],
) -> Vec<u8> {
    let mut head = BitWriter::default();
    head.bytes(&id.to_le_bytes());
    rect(&mut head, [0, 200, 0, 200]);
    rect(&mut head, [0, 400, 0, 400]);
    let mut styles = BitWriter::default();
    styles.bytes(&[1, 0x00]);
    styles.bytes(&start_color);
    styles.bytes(&end_color);
    styles.bytes(&[0]);
    write_records(&mut styles, 1, 0, start);
    let styles = styles.finish();
    let mut body = head.finish();
    body.extend_from_slice(&u32::try_from(styles.len()).unwrap_or(0).to_le_bytes());
    body.extend_from_slice(&styles);
    let mut end_bits = BitWriter::default();
    write_records(&mut end_bits, 0, 0, end);
    body.extend_from_slice(&end_bits.finish());
    body
}
