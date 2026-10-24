// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::context::Context;
use crate::look::Look;
use crate::paint::Area;
use crate::widget::Widget;

const MAX_SIDES: u32 = 64;

pub(super) fn paint(cx: &mut Context, widget: &Widget, look: &Look<'_>, area: Area, alpha: f32) {
    let style = &widget.shape;
    if !(3..=MAX_SIDES).contains(&style.sides) {
        cx.report.note(
            &widget.type_name,
            "polygon with fewer than 3 or more than 64 sides",
            &widget.name,
        );
        return;
    }
    let (middle_x, middle_y) = (area.x + area.width / 2.0, area.y + area.height / 2.0);
    let (rx, ry) = (area.width / 2.0, area.height / 2.0);
    let step = std::f32::consts::TAU / f32::from(u16::try_from(style.sides).unwrap_or(3));
    let start = style.start_angle.to_radians();
    let points: Vec<(f32, f32)> = (0..style.sides)
        .map(|corner| {
            let angle = step.mul_add(f32::from(u16::try_from(corner).unwrap_or(0)), start);
            (
                rx.mul_add(angle.cos(), middle_x),
                ry.mul_add(angle.sin(), middle_y),
            )
        })
        .collect();
    if style.do_fill
        && let Some(colour) = look.fill.and_then(|spec| cx.colour(spec, alpha, widget))
    {
        cx.painter.polygon(&points, colour);
    }
    if style.do_stroke
        && let Some(colour) = look.stroke.and_then(|spec| cx.colour(spec, alpha, widget))
    {
        cx.painter
            .stroke_polygon(&points, colour, style.stroke_width);
    }
}
