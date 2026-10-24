// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestInstance, TestValue};

/// A size of a synthetic widget along one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestDim {
    /// Pixels (or raw units in a raw canvas).
    Fixed(f32),
    /// A share of the parent's extent along the axis.
    Percent(f32),
    /// A share of the widget's own height.
    OfHeight(f32),
    /// A share of the widget's own width.
    OfWidth(f32),
    /// Sized by the content.
    Auto,
}

impl TestDim {
    pub(crate) fn value(self) -> TestValue {
        let (value, behavior) = match self {
            Self::Fixed(value) => (value, "Fixed"),
            Self::Percent(value) => (value, "Percent"),
            Self::OfHeight(value) => (value, "PercentOfY"),
            Self::OfWidth(value) => (value, "PercentOfX"),
            Self::Auto => (0.0, "Auto"),
        };
        TestValue::Struct(
            TestInstance::new("BuildingBlocks_FixedOrRelativeValue")
                .value("value", TestValue::Single(value))
                .value("behavior", TestValue::text(behavior)),
        )
    }
}

/// A colour of a synthetic widget or style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestColour {
    /// A solid colour, red, green, blue and alpha bytes.
    Solid([u8; 4]),
    /// A slot of the palette of the style in force, with an alpha factor.
    Slot(&'static str, f32),
}

impl TestColour {
    pub(crate) fn value(self) -> TestValue {
        TestValue::Struct(match self {
            Self::Solid([r, g, b, a]) => TestInstance::new("BuildingBlocks_ColorSolid").value(
                "color",
                TestValue::Struct(
                    TestInstance::new("SRGBA8")
                        .value("r", TestValue::UInt8(r))
                        .value("g", TestValue::UInt8(g))
                        .value("b", TestValue::UInt8(b))
                        .value("a", TestValue::UInt8(a)),
                ),
            ),
            Self::Slot(slot, alpha) => TestInstance::new("BuildingBlocks_ColorStyle")
                .value("color", TestValue::text(slot))
                .value("alpha", TestValue::Single(alpha)),
        })
    }
}
