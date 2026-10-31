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

/// An animation of `Alpha` from a value at the start of the timeline to another at its end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestAlphaAnimation {
    fill: &'static str,
    direction: &'static str,
    from: f32,
    to: f32,
    looping: bool,
    keyframes: bool,
    last_at: f32,
}

impl TestAlphaAnimation {
    /// Creates an animation with the fill mode (`None`, `Forward`, ...) and the direction
    /// (`Forward`, `Reverse`, ...), from `from` at 0 to `to` at 1.
    #[must_use]
    pub const fn alpha(fill: &'static str, direction: &'static str, from: f32, to: f32) -> Self {
        Self {
            fill,
            direction,
            from,
            to,
            looping: false,
            keyframes: true,
            last_at: 1.0,
        }
    }

    /// Makes the animation loop indefinitely.
    #[must_use]
    pub const fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    /// Leaves the timeline without keyframes.
    #[must_use]
    pub const fn without_keyframes(mut self) -> Self {
        self.keyframes = false;
        self
    }

    /// Puts the last keyframe at `percent` instead of 1.
    #[must_use]
    pub const fn ending_at(mut self, percent: f32) -> Self {
        self.last_at = percent;
        self
    }

    pub(crate) fn value(self) -> TestValue {
        let frames = if self.keyframes {
            vec![keyframe(0.0, self.from), keyframe(self.last_at, self.to)]
        } else {
            Vec::new()
        };
        TestValue::Struct(
            TestInstance::new("BuidlingBlocks_FieldModifierAnimation").value(
                "value",
                TestValue::Struct(
                    TestInstance::new("BuildingBlocks_Animation")
                        .value("fillMode", TestValue::text(self.fill))
                        .value("direction", TestValue::text(self.direction))
                        .value("loopIndefinitely", TestValue::Bool(self.looping))
                        .value(
                            "animationTimeline",
                            TestValue::Struct(
                                TestInstance::new("BuildingBlocks_TimelineTypeEmbedded")
                                    .value("keyframes", TestValue::Array(frames)),
                            ),
                        ),
                ),
            ),
        )
    }
}
