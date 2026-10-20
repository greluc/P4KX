use std::io::{self, Write};

use crate::geometry::Rgba;
use crate::place::ColorTransform;
use crate::raster_paint::GRADIENT_EXTENT;
use crate::scene::{
    BitmapPaint, Cap, GradientKind, GradientPaint, Join, LayerKind, Paint, PathCommand, ScenePath,
    ScenePoint, ShapeLayer, Spread, StrokeStyle,
};
use crate::svg_plan::{Fill, ImagePlan};
use crate::svg_text::{color, coordinate, fraction, matrix, number, opacity};

/// Where a layer sits in the document, for the ids of its paint servers.
#[derive(Clone, Copy)]
pub(crate) struct Place {
    pub(crate) group: usize,
    pub(crate) layer: usize,
}

/// How an element is painted.
enum Applied {
    Color(Rgba),
    Server { opacity: f32 },
}

/// Writes the ` d="…"` attribute of `path`.
pub(crate) fn path_data(out: &mut impl Write, path: &ScenePath) -> io::Result<()> {
    out.write_all(b" d=\"")?;
    for (at, command) in path.commands.iter().enumerate() {
        if at > 0 {
            out.write_all(b" ")?;
        }
        match *command {
            PathCommand::MoveTo(point) => {
                out.write_all(b"M")?;
                point_data(out, point)?;
            }
            PathCommand::LineTo(point) => {
                out.write_all(b"L")?;
                point_data(out, point)?;
            }
            PathCommand::QuadTo(control, point) => {
                out.write_all(b"Q")?;
                point_data(out, control)?;
                out.write_all(b" ")?;
                point_data(out, point)?;
            }
            PathCommand::Close => out.write_all(b"Z")?,
        }
    }
    out.write_all(b"\"")
}

fn point_data(out: &mut impl Write, point: ScenePoint) -> io::Result<()> {
    number(out, point.x, 3)?;
    out.write_all(b" ")?;
    number(out, point.y, 3)
}

/// Writes the elements of one layer, with the paint servers it needs, as drawn with the colour
/// transform `transform`.
pub(crate) fn write_layer(
    out: &mut impl Write,
    place: Place,
    layer: &ShapeLayer,
    transform: &ColorTransform,
    fill: Fill,
    images: &[ImagePlan],
) -> io::Result<()> {
    let applied = match (&layer.paint, fill) {
        (Paint::Solid(solid), _) => Applied::Color(transform.apply(*solid)),
        (Paint::Gradient(gradient), _) => {
            write_gradient(out, place, gradient, transform)?;
            Applied::Server { opacity: 1.0 }
        }
        (Paint::Bitmap(bitmap), Fill::Image { index, opacity }) => {
            let Some(image) = images.get(index) else {
                return Ok(());
            };
            if matches!(layer.kind, LayerKind::Fill) && !bitmap.repeating {
                return write_clipped_bitmap(out, place, layer, bitmap, index, opacity);
            }
            write_pattern(out, place, bitmap, index, image)?;
            Applied::Server { opacity }
        }
        (Paint::Bitmap(_), Fill::Plain) => return Ok(()),
    };
    out.write_all(b"<path")?;
    path_data(out, &layer.path)?;
    match &layer.kind {
        LayerKind::Fill => {
            paint(out, ("fill", "fill-opacity"), place, &applied)?;
            out.write_all(b" fill-rule=\"evenodd\"")?;
        }
        LayerKind::Stroke(style) => {
            out.write_all(b" fill=\"none\"")?;
            paint(out, ("stroke", "stroke-opacity"), place, &applied)?;
            stroke(out, style)?;
        }
    }
    out.write_all(b"/>\n")
}

/// Writes a layer drawn white, the form of the shapes of a mask.
pub(crate) fn write_mask_layer(out: &mut impl Write, layer: &ShapeLayer) -> io::Result<()> {
    out.write_all(b"<path")?;
    path_data(out, &layer.path)?;
    match &layer.kind {
        LayerKind::Fill => out.write_all(b" fill=\"#fff\" fill-rule=\"evenodd\"")?,
        LayerKind::Stroke(style) => {
            out.write_all(b" fill=\"none\" stroke=\"#fff\"")?;
            stroke(out, style)?;
        }
    }
    out.write_all(b"/>\n")
}

fn paint(
    out: &mut impl Write,
    (name, opacity_name): (&str, &str),
    place: Place,
    applied: &Applied,
) -> io::Result<()> {
    match applied {
        Applied::Color(value) => {
            color(out, name, *value)?;
            opacity(out, opacity_name, fraction(value.alpha))
        }
        Applied::Server { opacity: value } => {
            write!(out, " {name}=\"url(#p{}_{})\"", place.group, place.layer)?;
            opacity(out, opacity_name, *value)
        }
    }
}

fn stroke(out: &mut impl Write, style: &StrokeStyle) -> io::Result<()> {
    out.write_all(b" stroke-width=\"")?;
    if style.width > 0.0 {
        number(out, style.width, 3)?;
        out.write_all(b"\"")?;
    } else {
        out.write_all(b"1\" vector-effect=\"non-scaling-stroke\"")?;
    }
    out.write_all(match style.cap {
        Cap::Round => b" stroke-linecap=\"round\"".as_slice(),
        Cap::Butt => b" stroke-linecap=\"butt\"".as_slice(),
        Cap::Square => b" stroke-linecap=\"square\"".as_slice(),
    })?;
    out.write_all(match style.join {
        Join::Round => b" stroke-linejoin=\"round\"".as_slice(),
        Join::Bevel => b" stroke-linejoin=\"bevel\"".as_slice(),
        Join::Miter => b" stroke-linejoin=\"miter\"".as_slice(),
    })?;
    out.write_all(b" stroke-miterlimit=\"")?;
    number(out, style.miter_limit.max(1.0), 3)?;
    out.write_all(b"\"")
}

fn write_gradient(
    out: &mut impl Write,
    place: Place,
    gradient: &GradientPaint,
    transform: &ColorTransform,
) -> io::Result<()> {
    let tag = match gradient.kind {
        GradientKind::Linear => "linearGradient",
        GradientKind::Radial | GradientKind::Focal(_) => "radialGradient",
    };
    write!(
        out,
        "<{tag} id=\"p{}_{}\" gradientUnits=\"userSpaceOnUse\"",
        place.group, place.layer
    )?;
    match gradient.kind {
        GradientKind::Linear => {
            out.write_all(b" x1=\"-16384\" y1=\"0\" x2=\"16384\" y2=\"0\"")?;
        }
        GradientKind::Radial | GradientKind::Focal(_) => {
            let focal = match gradient.kind {
                GradientKind::Focal(focal) => focal * GRADIENT_EXTENT,
                _ => 0.0,
            };
            out.write_all(b" cx=\"0\" cy=\"0\" r=\"16384\"")?;
            coordinate(out, "fx", focal)?;
            out.write_all(b" fy=\"0\"")?;
        }
    }
    matrix(out, "gradientTransform", &gradient.transform)?;
    out.write_all(match gradient.spread {
        Spread::Pad => b"".as_slice(),
        Spread::Reflect => b" spreadMethod=\"reflect\"".as_slice(),
        Spread::Repeat => b" spreadMethod=\"repeat\"".as_slice(),
    })?;
    out.write_all(b">\n")?;
    for stop in &gradient.stops {
        let stop_color = transform.apply(stop.color);
        out.write_all(b"<stop offset=\"")?;
        number(out, f32::from(stop.ratio) / 255.0, 4)?;
        out.write_all(b"\"")?;
        color(out, "stop-color", stop_color)?;
        opacity(out, "stop-opacity", fraction(stop_color.alpha))?;
        out.write_all(b"/>\n")?;
    }
    writeln!(out, "</{tag}>")
}

fn write_pattern(
    out: &mut impl Write,
    place: Place,
    bitmap: &BitmapPaint,
    index: usize,
    image: &ImagePlan,
) -> io::Result<()> {
    write!(
        out,
        "<pattern id=\"p{}_{}\" patternUnits=\"userSpaceOnUse\" width=\"{}\" height=\"{}\"",
        place.group, place.layer, image.width, image.height
    )?;
    matrix(out, "patternTransform", &bitmap.transform)?;
    write!(out, "><use xlink:href=\"#i{index}\"")?;
    rendering(out, bitmap)?;
    out.write_all(b"/></pattern>\n")
}

fn write_clipped_bitmap(
    out: &mut impl Write,
    place: Place,
    layer: &ShapeLayer,
    bitmap: &BitmapPaint,
    index: usize,
    value: f32,
) -> io::Result<()> {
    write!(
        out,
        "<clipPath id=\"c{}_{}\"><path",
        place.group, place.layer
    )?;
    path_data(out, &layer.path)?;
    out.write_all(b" clip-rule=\"evenodd\"/></clipPath>\n")?;
    write!(
        out,
        "<g clip-path=\"url(#c{}_{})\"><use xlink:href=\"#i{index}\"",
        place.group, place.layer
    )?;
    matrix(out, "transform", &bitmap.transform)?;
    opacity(out, "opacity", value)?;
    rendering(out, bitmap)?;
    out.write_all(b"/></g>\n")
}

fn rendering(out: &mut impl Write, bitmap: &BitmapPaint) -> io::Result<()> {
    if bitmap.smoothed {
        Ok(())
    } else {
        out.write_all(b" image-rendering=\"optimizeSpeed\"")
    }
}
