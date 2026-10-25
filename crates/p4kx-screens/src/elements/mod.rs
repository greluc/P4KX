// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod canvas;
mod circle;
mod display;
mod polygon;
mod text;
mod unsupported;

pub(crate) use canvas::nested as nested_canvas;

use crate::context::Context;
use crate::look::Look;
use crate::paint::Area;
use crate::text::{TEXT, TEXT_FIELD};
use crate::widget::Widget;

const STRUCTURAL: [&str; 8] = [
    "BuildingBlocks_DisplayWidget",
    "BuildingBlocks_WidgetBase",
    "BuildingBlocks_WidgetCard",
    "BuildingBlocks_WidgetContainer",
    "BuildingBlocks_WidgetCanvas",
    "BuildingBlocks_WidgetBodyBackground",
    "BuildingBlocks_WidgetClone",
    "BuildingBlocks_WidgetList",
];

pub(crate) const CIRCLE: &str = "BuildingBlocks_WidgetCircle";
pub(crate) const POLYGON: &str = "BuildingBlocks_WidgetPolygon";

pub(crate) fn is_rendered(type_name: &str) -> bool {
    STRUCTURAL.contains(&type_name) || matches!(type_name, CIRCLE | POLYGON | TEXT | TEXT_FIELD)
}

pub(crate) fn paint(
    cx: &mut Context<'_>,
    widget: &Widget,
    look: &Look<'_>,
    area: Area,
    alpha: f32,
) {
    display::paint(cx, widget, look, area, alpha);
    match widget.type_name.as_str() {
        CIRCLE => circle::paint(cx, widget, look, area, alpha),
        POLYGON => polygon::paint(cx, widget, look, area, alpha),
        TEXT | TEXT_FIELD => text::paint(cx, widget, look, area, alpha),
        name if STRUCTURAL.contains(&name) => {}
        _ => unsupported::note(cx, widget),
    }
}
