// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::context::Context;
use crate::look::Look;
use crate::paint::Area;
use crate::widget::Widget;

pub(super) fn paint(cx: &mut Context, widget: &Widget, look: &Look<'_>, area: Area, alpha: f32) {
    let style = &widget.shape;
    if !style.do_fill && !style.do_stroke {
        cx.report.note(
            &widget.type_name,
            "circle without fill and stroke",
            &widget.name,
        );
    }
    if style.do_fill
        && let Some(colour) = look.fill.and_then(|spec| cx.colour(spec, alpha, widget))
    {
        cx.painter.ellipse(area, colour);
    }
    if style.do_stroke
        && let Some(colour) = look.stroke.and_then(|spec| cx.colour(spec, alpha, widget))
    {
        cx.painter.stroke_ellipse(area, colour, style.stroke_width);
    }
}
