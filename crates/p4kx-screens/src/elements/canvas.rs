// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{DataForge, Guid};

use super::standard;
use crate::widget::Widget;

const WIDGET_CANVAS: &str = "BuildingBlocks_WidgetCanvas";

/// Returns the canvas a widget shows inside its box: `chosen`, the one a canvas widget names
/// once styles and bindings had their say, or the canvas that holds the look of a standard
/// widget.
pub(crate) fn nested(widget: &Widget, chosen: Option<Guid>, forge: &DataForge<'_>) -> Option<Guid> {
    if widget.type_name == WIDGET_CANVAS {
        chosen
    } else {
        standard::canvas_of(widget, forge)
    }
}
