// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::{image, unsupported};
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
    if widget.picture.url.is_empty() {
        cx.report.note(
            &widget.type_name,
            "the texture is chosen by the game at run time",
            &widget.name,
        );
        return;
    }
    if !cx.images.is_enabled() {
        cx.report.images_left_out += 1;
        unsupported::note(cx, widget);
        return;
    }
    if !area.is_empty() {
        image::draw(cx, widget, look, area, alpha, &widget.picture.url);
    }
}
