// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::{Affine, LayerKind, Paint, PathCommand, Rgba, ScenePath, ShapeLayer, StrokeStyle};

use super::Walk;
use super::number::MAX_MAGNITUDE;
use super::style::{Colour, Ink, Style};

fn rgba(colour: Colour, factor: f32) -> Rgba {
    let [red, green, blue] = colour.rgb;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the product is clamped to 0 to 255"
    )]
    let alpha = (colour.alpha * factor * 255.0).clamp(0.0, 255.0).round() as u8;
    Rgba {
        red,
        green,
        blue,
        alpha,
    }
}

fn transformed(commands: &[PathCommand], to: &Affine) -> Option<ScenePath> {
    let map = |p: p4kx_gfx::ScenePoint| p4kx_gfx::ScenePoint {
        x: to.a.mul_add(p.x, to.c.mul_add(p.y, to.tx)),
        y: to.b.mul_add(p.x, to.d.mul_add(p.y, to.ty)),
    };
    let mapped: Vec<PathCommand> = commands
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(p) => PathCommand::MoveTo(map(p)),
            PathCommand::LineTo(p) => PathCommand::LineTo(map(p)),
            PathCommand::QuadTo(c, p) => PathCommand::QuadTo(map(c), map(p)),
            PathCommand::Close => PathCommand::Close,
        })
        .collect();
    let sane = mapped.iter().all(|command| match command {
        PathCommand::MoveTo(p) | PathCommand::LineTo(p) => point_ok(*p),
        PathCommand::QuadTo(c, p) => point_ok(*c) && point_ok(*p),
        PathCommand::Close => true,
    });
    sane.then_some(ScenePath { commands: mapped })
}

fn point_ok(p: p4kx_gfx::ScenePoint) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= MAX_MAGNITUDE && p.y.abs() <= MAX_MAGNITUDE
}

pub(super) fn emit(walk: &mut Walk<'_, '_>, commands: &[PathCommand], style: &Style, to: &Affine) {
    if commands.is_empty() {
        return;
    }
    let Some(path) = transformed(commands, to) else {
        walk.note("a path with coordinates out of range is left out");
        return;
    };
    if let Ink::Colour(fill) = style.fill {
        let colour = rgba(fill, style.fill_opacity * style.opacity);
        if colour.alpha > 0 {
            walk.slots.push(style.slot.clone());
            walk.layers.push(ShapeLayer {
                path: path.clone(),
                paint: Paint::Solid(colour),
                kind: LayerKind::Fill,
            });
        }
    }
    if let Ink::Colour(stroke) = style.stroke {
        let colour = rgba(stroke, style.stroke_opacity * style.opacity);
        let scale = to.b.mul_add(-to.c, to.a * to.d).abs().sqrt();
        let width = style.stroke_width * scale;
        if colour.alpha > 0 && width > 0.0 && width.is_finite() {
            walk.slots.push(style.slot.clone());
            walk.layers.push(ShapeLayer {
                path,
                paint: Paint::Solid(colour),
                kind: LayerKind::Stroke(StrokeStyle {
                    width,
                    cap: style.cap,
                    join: style.join,
                    miter_limit: style.miter_limit,
                }),
            });
        }
    }
}
