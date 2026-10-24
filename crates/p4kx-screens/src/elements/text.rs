// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::unsupported;
use crate::context::Context;
use crate::look::Look;
use crate::paint::Area;
use crate::widget::Widget;

pub(super) fn paint(
    cx: &mut Context<'_>,
    widget: &Widget,
    look: &Look<'_>,
    area: Area,
    alpha: f32,
) {
    if !cx.texts.is_enabled() {
        cx.report.texts_left_out += 1;
        unsupported::note(cx, widget);
        return;
    }
    let content = area.inset(
        widget.padding.top,
        widget.padding.right,
        widget.padding.bottom,
        widget.padding.left,
    );
    cx.text(widget, look, content, alpha);
}
