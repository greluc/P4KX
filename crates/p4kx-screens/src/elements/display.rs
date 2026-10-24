// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::boxes::paint_box;
use crate::context::Context;
use crate::look::Look;
use crate::paint::Area;
use crate::widget::Widget;

pub(super) fn paint(cx: &mut Context, widget: &Widget, look: &Look<'_>, area: Area, alpha: f32) {
    paint_box(cx, widget, look, area, alpha);
}
