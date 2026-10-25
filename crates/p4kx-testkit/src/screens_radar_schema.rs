// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestStruct, TestType};

pub(crate) fn radar_structs() -> Vec<TestStruct> {
    let display = "BuildingBlocks_DisplayWidget";
    vec![
        TestStruct::new("BuildingBlocks_WidgetSeparator")
            .parent(display)
            .field("direction", TestType::String)
            .field("style", TestType::String),
        TestStruct::new("BuildingBlocks_WidgetRuntimeImage")
            .parent(display)
            .field("imageSource", TestType::String),
        TestStruct::new("BuildingBlocks_WidgetLine")
            .parent(display)
            .field("strokeWidth", TestType::Single),
        TestStruct::new("BuildingBlocks_GeometryEntityWidget")
            .parent(display)
            .field("geometryPath", TestType::String),
        TestStruct::new("BuildingBlocks_WidgetWindow")
            .parent(display)
            .field("renderType", TestType::String),
        TestStruct::new("BuildingBlocks_WidgetLight").parent(display),
        TestStruct::new("BuildingBlocks_WidgetEditBox")
            .parent(display)
            .field("helperString", TestType::String),
    ]
}
