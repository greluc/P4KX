// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::context::Context;
use crate::widget::Widget;

pub(crate) const RUNTIME_IMAGE: &str = "BuildingBlocks_WidgetRuntimeImage";
pub(crate) const LINE: &str = "BuildingBlocks_WidgetLine";

const SCENES: [&str; 5] = [
    "BuildingBlocks_GeometryEntityWidget",
    "BuildingBlocks_VehicleEntityWidget",
    "BuildingBlocks_WidgetHoloVolume",
    "BuildingBlocks_WidgetWindow",
    "BuildingBlocks_WidgetDustVolume",
];

const SILENT: [&str; 3] = [
    "BuildingBlocks_WidgetLight",
    "BuildingBlocks_WidgetLighting",
    "BuildingBlocks_WidgetEditBox",
];

pub(crate) fn is_scene(type_name: &str) -> bool {
    SCENES.contains(&type_name)
}

/// Returns whether a widget has nothing of its own to draw: a light, or an edit box whose text
/// is the widget it names.
pub(crate) fn is_silent(type_name: &str) -> bool {
    SILENT.contains(&type_name)
}

/// Lists a widget the game fills with a picture it renders at run time, such as a map; the box
/// of the widget is drawn as for every widget.
pub(super) fn image(cx: &mut Context<'_>, widget: &Widget) {
    cx.report.images_left_out += 1;
    cx.report.note(
        &widget.type_name,
        "the image is rendered by the game at run time",
        &widget.name,
    );
}

/// Lists a widget that shows 3D content the game draws at run time.
pub(super) fn scene(cx: &mut Context<'_>, widget: &Widget) {
    cx.report.images_left_out += 1;
    cx.report.note(
        &widget.type_name,
        "3D content is drawn by the game at run time",
        &widget.name,
    );
}

/// Lists a line whose far end is a widget the game places at run time.
pub(super) fn line(cx: &mut Context<'_>, widget: &Widget) {
    cx.report.note(
        &widget.type_name,
        "the line ends at a widget the game places at run time",
        &widget.name,
    );
}
