// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::Guid;

use crate::widget::Widget;

const WIDGET_CANVAS: &str = "BuildingBlocks_WidgetCanvas";

pub(crate) fn nested(widget: &Widget) -> Option<Guid> {
    if widget.type_name == WIDGET_CANVAS {
        widget.canvas
    } else {
        None
    }
}
