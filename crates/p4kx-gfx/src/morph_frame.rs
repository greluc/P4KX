// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::geometry::{Matrix, Point, Rgba};
use crate::morph::MorphShape;
use crate::shape::{ShapeRecord, StyleChange};
use crate::style::{FillStyle, Gradient, GradientStop, LineStyle};

/// The styles and records of a morph shape at `ratio`, from 0 (start) to 65535 (end).
pub(crate) fn morph_at(
    morph: &MorphShape,
    ratio: u16,
) -> (Vec<FillStyle>, Vec<LineStyle>, Vec<ShapeRecord>) {
    let t = f32::from(ratio) / f32::from(u16::MAX);
    let fills = morph
        .start_fill_styles
        .iter()
        .enumerate()
        .map(|(at, start)| {
            morph
                .end_fill_styles
                .get(at)
                .map_or_else(|| start.clone(), |end| fill(start, end, t))
        })
        .collect();
    let lines = morph
        .start_line_styles
        .iter()
        .enumerate()
        .map(|(at, start)| {
            morph
                .end_line_styles
                .get(at)
                .map_or_else(|| start.clone(), |end| line(start, end, t))
        })
        .collect();
    (fills, lines, records(morph, t))
}

fn records(morph: &MorphShape, t: f32) -> Vec<ShapeRecord> {
    let mut end = morph.end_records.iter().peekable();
    let mut records = Vec::with_capacity(morph.start_records.len());
    for record in &morph.start_records {
        match record {
            ShapeRecord::StyleChange(change) => {
                let end_move = match end.peek() {
                    Some(ShapeRecord::StyleChange(end_change)) => {
                        let end_move = end_change.move_to;
                        end.next();
                        end_move
                    }
                    _ => None,
                };
                let move_to = change
                    .move_to
                    .map(|start| end_move.map_or(start, |end_point| point(start, end_point, t)));
                records.push(ShapeRecord::StyleChange(Box::new(StyleChange {
                    move_to,
                    ..(**change).clone()
                })));
            }
            edge => {
                while matches!(end.peek(), Some(ShapeRecord::StyleChange(_))) {
                    end.next();
                }
                records.push(
                    end.next()
                        .map_or_else(|| edge.clone(), |end| lerp_edge(edge, end, t)),
                );
            }
        }
    }
    records
}

const fn as_curve(record: &ShapeRecord) -> Option<(Point, Point)> {
    match *record {
        ShapeRecord::StraightEdge { delta } => {
            let half = Point {
                x: delta.x / 2,
                y: delta.y / 2,
            };
            Some((
                half,
                Point {
                    x: delta.x - half.x,
                    y: delta.y - half.y,
                },
            ))
        }
        ShapeRecord::CurvedEdge { control, anchor } => Some((control, anchor)),
        ShapeRecord::StyleChange(_) => None,
    }
}

fn lerp_edge(start: &ShapeRecord, end: &ShapeRecord, t: f32) -> ShapeRecord {
    if let (ShapeRecord::StraightEdge { delta: a }, ShapeRecord::StraightEdge { delta: b }) =
        (start, end)
    {
        return ShapeRecord::StraightEdge {
            delta: point(*a, *b, t),
        };
    }
    match (as_curve(start), as_curve(end)) {
        (Some((start_control, start_anchor)), Some((end_control, end_anchor))) => {
            ShapeRecord::CurvedEdge {
                control: point(start_control, end_control, t),
                anchor: point(start_anchor, end_anchor, t),
            }
        }
        _ => start.clone(),
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    (end - start).mul_add(t, start)
}

fn whole(start: i32, end: i32, t: f32) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "the value lies between two i32 values"
    )]
    let value = lerp(start as f32, end as f32, t).round() as i32;
    value
}

fn byte(start: u8, end: u8, t: f32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value lies between two bytes"
    )]
    let value = lerp(f32::from(start), f32::from(end), t)
        .round()
        .clamp(0.0, 255.0) as u8;
    value
}

fn point(start: Point, end: Point, t: f32) -> Point {
    Point {
        x: whole(start.x, end.x, t),
        y: whole(start.y, end.y, t),
    }
}

fn color(start: Rgba, end: Rgba, t: f32) -> Rgba {
    Rgba {
        red: byte(start.red, end.red, t),
        green: byte(start.green, end.green, t),
        blue: byte(start.blue, end.blue, t),
        alpha: byte(start.alpha, end.alpha, t),
    }
}

fn matrix(start: &Matrix, end: &Matrix, t: f32) -> Matrix {
    Matrix {
        scale_x: lerp(start.scale_x, end.scale_x, t),
        scale_y: lerp(start.scale_y, end.scale_y, t),
        rotate_skew0: lerp(start.rotate_skew0, end.rotate_skew0, t),
        rotate_skew1: lerp(start.rotate_skew1, end.rotate_skew1, t),
        translate_x: whole(start.translate_x, end.translate_x, t),
        translate_y: whole(start.translate_y, end.translate_y, t),
    }
}

fn gradient(start: &Gradient, end: &Gradient, t: f32) -> Gradient {
    Gradient {
        stops: start
            .stops
            .iter()
            .zip(&end.stops)
            .map(|(a, b)| GradientStop {
                ratio: byte(a.ratio, b.ratio, t),
                color: color(a.color, b.color, t),
            })
            .collect(),
        spread: start.spread,
        interpolation: start.interpolation,
    }
}

fn fill(start: &FillStyle, end: &FillStyle, t: f32) -> FillStyle {
    match (start, end) {
        (FillStyle::Solid { color: a }, FillStyle::Solid { color: b }) => FillStyle::Solid {
            color: color(*a, *b, t),
        },
        (
            FillStyle::Bitmap {
                bitmap_id,
                matrix: am,
                repeating,
                smoothed,
            },
            FillStyle::Bitmap { matrix: bm, .. },
        ) => FillStyle::Bitmap {
            bitmap_id: *bitmap_id,
            matrix: matrix(am, bm, t),
            repeating: *repeating,
            smoothed: *smoothed,
        },
        _ => gradient_fill(start, end, t).unwrap_or_else(|| start.clone()),
    }
}

const fn gradient_parts(style: &FillStyle) -> Option<(&Matrix, &Gradient, f32)> {
    match style {
        FillStyle::LinearGradient { matrix, gradient }
        | FillStyle::RadialGradient { matrix, gradient } => Some((matrix, gradient, 0.0)),
        FillStyle::FocalGradient {
            matrix,
            gradient,
            focal_point,
        } => Some((matrix, gradient, *focal_point)),
        FillStyle::Solid { .. } | FillStyle::Bitmap { .. } => None,
    }
}

fn gradient_fill(start: &FillStyle, end: &FillStyle, t: f32) -> Option<FillStyle> {
    let (start_matrix, start_gradient, start_focal) = gradient_parts(start)?;
    let (end_matrix, end_gradient, end_focal) = gradient_parts(end)?;
    let matrix = matrix(start_matrix, end_matrix, t);
    let gradient = gradient(start_gradient, end_gradient, t);
    match (start, end) {
        (FillStyle::LinearGradient { .. }, FillStyle::LinearGradient { .. }) => {
            Some(FillStyle::LinearGradient { matrix, gradient })
        }
        (FillStyle::RadialGradient { .. }, FillStyle::RadialGradient { .. }) => {
            Some(FillStyle::RadialGradient { matrix, gradient })
        }
        (FillStyle::FocalGradient { .. }, FillStyle::FocalGradient { .. }) => {
            Some(FillStyle::FocalGradient {
                matrix,
                gradient,
                focal_point: lerp(start_focal, end_focal, t),
            })
        }
        _ => None,
    }
}
fn line(start: &LineStyle, end: &LineStyle, t: f32) -> LineStyle {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the width lies between two u16 values"
    )]
    let width = lerp(f32::from(start.width), f32::from(end.width), t).round() as u16;
    LineStyle {
        width,
        color: color(start.color, end.color, t),
        fill: match (&start.fill, &end.fill) {
            (Some(a), Some(b)) => Some(fill(a, b, t)),
            _ => start.fill.clone(),
        },
        start_cap: start.start_cap,
        end_cap: start.end_cap,
        join: start.join,
        miter_limit: start.miter_limit,
        no_close: start.no_close,
    }
}
