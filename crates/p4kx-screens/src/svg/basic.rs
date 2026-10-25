// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::PathCommand;
use roxmltree::Node;

use super::SvgError;
use super::arc::ArcShape;
use super::number::{leading_number, number_list};
use super::path::{Builder, parse};

const MAX_POINTS: usize = 1 << 16;

fn length(node: Node<'_, '_>, name: &str) -> f32 {
    node.attribute(name).and_then(leading_number).unwrap_or(0.0)
}

fn points(node: Node<'_, '_>) -> Result<Vec<(f32, f32)>, SvgError> {
    let values = node
        .attribute("points")
        .and_then(|text| number_list(text, MAX_POINTS * 2))
        .ok_or(SvgError::Path)?;
    Ok(values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[x, y]| (*x, *y))
        .collect())
}

const fn quarter(rx: f32, ry: f32) -> ArcShape {
    ArcShape {
        radii: (rx, ry),
        turn_degrees: 0.0,
        large: false,
        sweep: true,
    }
}

fn rounded(
    out: &mut Builder<'_>,
    (x, y): (f32, f32),
    (w, h): (f32, f32),
    (rx, ry): (f32, f32),
) -> Result<(), SvgError> {
    let (rx, ry) = (rx.min(w / 2.0), ry.min(h / 2.0));
    out.move_to((x + rx, y))?;
    out.line_to((x + w - rx, y))?;
    out.arc_to(&quarter(rx, ry), (x + w, y + ry))?;
    out.line_to((x + w, y + h - ry))?;
    out.arc_to(&quarter(rx, ry), (x + w - rx, y + h))?;
    out.line_to((x + rx, y + h))?;
    out.arc_to(&quarter(rx, ry), (x, y + h - ry))?;
    out.line_to((x, y + ry))?;
    out.arc_to(&quarter(rx, ry), (x + rx, y))?;
    out.close()
}

fn ellipse(
    out: &mut Builder<'_>,
    (cx, cy): (f32, f32),
    (rx, ry): (f32, f32),
) -> Result<(), SvgError> {
    out.move_to((cx + rx, cy))?;
    out.arc_to(&quarter(rx, ry), (cx, cy + ry))?;
    out.arc_to(&quarter(rx, ry), (cx - rx, cy))?;
    out.arc_to(&quarter(rx, ry), (cx, cy - ry))?;
    out.arc_to(&quarter(rx, ry), (cx + rx, cy))?;
    out.close()
}

/// Returns the commands of a basic shape or path element, `None` for any other element.
pub(super) fn commands(
    node: Node<'_, '_>,
    budget: &mut usize,
) -> Result<Option<Vec<PathCommand>>, SvgError> {
    let tag = node.tag_name().name();
    if tag == "path" {
        let data = node.attribute("d").unwrap_or_default();
        return parse(data, budget).map(Some);
    }
    let mut out = Builder::new(budget);
    match tag {
        "rect" => {
            let (w, h) = (length(node, "width"), length(node, "height"));
            let (mut rx, mut ry) = (length(node, "rx"), length(node, "ry"));
            if rx <= 0.0 {
                rx = ry;
            }
            if ry <= 0.0 {
                ry = rx;
            }
            if w <= 0.0 || h <= 0.0 {
                return Ok(Some(Vec::new()));
            }
            let at = (length(node, "x"), length(node, "y"));
            if rx > 0.0 && ry > 0.0 {
                rounded(&mut out, at, (w, h), (rx, ry))?;
            } else {
                out.move_to(at)?;
                out.line_to((at.0 + w, at.1))?;
                out.line_to((at.0 + w, at.1 + h))?;
                out.line_to((at.0, at.1 + h))?;
                out.close()?;
            }
        }
        "circle" | "ellipse" => {
            let (rx, ry) = if tag == "circle" {
                (length(node, "r"), length(node, "r"))
            } else {
                (length(node, "rx"), length(node, "ry"))
            };
            if rx <= 0.0 || ry <= 0.0 {
                return Ok(Some(Vec::new()));
            }
            ellipse(&mut out, (length(node, "cx"), length(node, "cy")), (rx, ry))?;
        }
        "line" => {
            out.move_to((length(node, "x1"), length(node, "y1")))?;
            out.line_to((length(node, "x2"), length(node, "y2")))?;
        }
        "polyline" | "polygon" => {
            let mut corners = points(node)?.into_iter();
            if let Some(first) = corners.next() {
                out.move_to(first)?;
                for corner in corners {
                    out.line_to(corner)?;
                }
                if tag == "polygon" {
                    out.close()?;
                }
            }
        }
        _ => return Ok(None),
    }
    Ok(Some(out.commands))
}
