// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestInstance, TestStruct, TestType, TestValue};

const MODIFIER: &str = "BuildingBlocks_FieldModifier";

fn class(name: &str) -> TestType {
    TestType::Class(name.to_owned())
}

/// Adds the structs of the animation modifier of a style entry, as the game names them.
pub(crate) fn animation_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("BuidlingBlocks_FieldModifierAnimation")
            .parent(MODIFIER)
            .field("value", class("BuildingBlocks_Animation")),
        TestStruct::new("BuildingBlocks_Animation")
            .field("fillMode", TestType::String)
            .field("direction", TestType::String)
            .field("loopIndefinitely", TestType::Bool)
            .field(
                "animationTimeline",
                class("BuildingBlocks_TimelineTypeEmbedded"),
            ),
        TestStruct::new("BuildingBlocks_TimelineTypeEmbedded")
            .array("keyframes", class("BuildingBlocks_Keyframe")),
        TestStruct::new("BuildingBlocks_Keyframe")
            .field("percent", TestType::Single)
            .array("modifiers", class("BuildingBlocks_KeyframeModifierData")),
        TestStruct::new("BuildingBlocks_KeyframeModifierData")
            .field("modifier", TestType::StrongPointer(MODIFIER.to_owned())),
    ]
}

fn alpha(value: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("BuildingBlocks_FieldModifierNumber")
            .value("field", TestValue::text("Alpha"))
            .value("value", TestValue::Single(value)),
    )
}

fn keyframe(percent: f32, value: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("BuildingBlocks_Keyframe")
            .value("percent", TestValue::Single(percent))
            .value(
                "modifiers",
                TestValue::Array(vec![TestValue::Struct(
                    TestInstance::new("BuildingBlocks_KeyframeModifierData")
                        .value("modifier", alpha(value)),
                )]),
            ),
    )
}

/// Returns an animation of `Alpha` from `from` at 0 to `to` at 1, with the fill mode and the
/// direction given.
pub(crate) fn alpha_animation(fill: &str, direction: &str, (from, to): (f32, f32)) -> TestValue {
    TestValue::Struct(
        TestInstance::new("BuidlingBlocks_FieldModifierAnimation").value(
            "value",
            TestValue::Struct(
                TestInstance::new("BuildingBlocks_Animation")
                    .value("fillMode", TestValue::text(fill))
                    .value("direction", TestValue::text(direction))
                    .value("loopIndefinitely", TestValue::Bool(false))
                    .value(
                        "animationTimeline",
                        TestValue::Struct(
                            TestInstance::new("BuildingBlocks_TimelineTypeEmbedded").value(
                                "keyframes",
                                TestValue::Array(vec![keyframe(0.0, from), keyframe(1.0, to)]),
                            ),
                        ),
                    ),
            ),
        ),
    )
}
