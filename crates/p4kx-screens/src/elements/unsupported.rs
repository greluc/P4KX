// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::context::Context;
use crate::widget::Widget;

pub(super) fn note(cx: &mut Context, widget: &Widget) {
    cx.report.note(
        &widget.type_name,
        "no renderer for this element type",
        &widget.name,
    );
}
