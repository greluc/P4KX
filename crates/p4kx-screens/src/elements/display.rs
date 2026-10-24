// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::context::Context;
use crate::look::Look;
use crate::paint::Area;
use crate::widget::Widget;

pub(super) fn paint(cx: &mut Context, widget: &Widget, look: &Look<'_>, area: Area, alpha: f32) {
    if let Some(colour) = look
        .background
        .and_then(|spec| cx.colour(spec, alpha, widget))
    {
        cx.painter.rect(area, colour);
    }
    let [(top, _), (right, _), (bottom, _), (left, _)] = look.borders;
    let strips = [
        Area {
            height: top,
            ..area
        },
        Area {
            x: area.x + area.width - right,
            width: right,
            ..area
        },
        Area {
            y: area.y + area.height - bottom,
            height: bottom,
            ..area
        },
        Area {
            width: left,
            ..area
        },
    ];
    for (strip, (width, spec)) in strips.into_iter().zip(look.borders) {
        if width > 0.0
            && let Some(colour) = spec.and_then(|spec| cx.colour(spec, alpha, widget))
        {
            cx.painter.rect(strip, colour);
        }
    }
}
