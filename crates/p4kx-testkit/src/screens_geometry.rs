// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::screens_widget::{TestColour, TestDim, vec3};
use crate::{TestInstance, TestModifier, TestStruct, TestType, TestValue, TestWidget};

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn weak_widget() -> TestType {
    TestType::WeakPointer("BuildingBlocks_DisplayWidget".to_owned())
}

fn class(name: &str) -> TestType {
    TestType::Class(name.to_owned())
}

/// Adds the structs of the background and the border of a box, with its rounded corners.
pub(crate) fn box_structs() -> Vec<TestStruct> {
    let corner = || class("BuildingBlocks_BorderRadiusCorner");
    vec![
        TestStruct::new("BuildingBlocks_Background")
            .field("enable", TestType::Bool)
            .field("color", pointer("BuildingBlocks_Color"))
            .field("topLeftColor", pointer("BuildingBlocks_Color"))
            .field("topRightColor", pointer("BuildingBlocks_Color"))
            .field("bottomLeftColor", pointer("BuildingBlocks_Color"))
            .field("bottomRightCColor", pointer("BuildingBlocks_Color")),
        TestStruct::new("BuildingBlocks_BorderSide")
            .field("width", TestType::Single)
            .field("color", pointer("BuildingBlocks_Color")),
        TestStruct::new("BuildingBlocks_Border")
            .field("top", class("BuildingBlocks_BorderSide"))
            .field("right", class("BuildingBlocks_BorderSide"))
            .field("bottom", class("BuildingBlocks_BorderSide"))
            .field("left", class("BuildingBlocks_BorderSide"))
            .field("topLeftRadius", corner())
            .field("topRightRadius", corner())
            .field("bottomRightRadius", corner())
            .field("bottomLeftRadius", corner()),
        TestStruct::new("BuildingBlocks_BorderRadiusCorner")
            .field("chamfer", TestType::Bool)
            .field("radius", class("BuildingBlocks_FixedOrRelativeValue")),
    ]
}

/// Adds the structs of clones, lists, the modifiers of clones and the static array variables of
/// canvases.
pub(crate) fn instance_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("BuildingBlocks_WidgetClone")
            .parent("BuildingBlocks_DisplayWidget")
            .field("target", weak_widget())
            .array("modifiers", class("BuildingBlocks_FieldModifierPair")),
        TestStruct::new("BuildingBlocks_WidgetList")
            .parent("BuildingBlocks_DisplayWidget")
            .field("target", weak_widget())
            .field("arrayVariable", TestType::String),
        TestStruct::new("BuildingBlocks_FieldModifierPair")
            .field("target", weak_widget())
            .field("modifier", pointer("BuildingBlocks_FieldModifier")),
        TestStruct::new("BuildingBlocks_StaticVariable").field("name", TestType::String),
        TestStruct::new("BuildingBlocks_StaticVariableArray")
            .parent("BuildingBlocks_StaticVariable")
            .array("value", pointer("BuildingBlocks_StaticArrayEntry")),
        TestStruct::new("BuildingBlocks_StaticArrayEntry").array(
            "structureVariables",
            pointer("BuildingBlocks_StaticArrayData"),
        ),
        TestStruct::new("BuildingBlocks_StaticArrayData").field("name", TestType::String),
        TestStruct::new("BuildingBlocks_StaticArrayDataLocalization")
            .parent("BuildingBlocks_StaticArrayData")
            .array("values", TestType::Locale),
    ]
}

pub(crate) fn static_arrays(arrays: &[(String, usize)]) -> TestValue {
    TestValue::Array(
        arrays
            .iter()
            .map(|(name, rows)| {
                let column = TestInstance::new("BuildingBlocks_StaticArrayDataLocalization")
                    .value("name", TestValue::text("column"))
                    .value(
                        "values",
                        TestValue::Array(
                            (0..*rows)
                                .map(|row| TestValue::text(format!("@row{row}")))
                                .collect(),
                        ),
                    );
                let entry = TestInstance::new("BuildingBlocks_StaticArrayEntry").value(
                    "structureVariables",
                    TestValue::Array(vec![TestValue::Struct(column)]),
                );
                TestValue::Struct(
                    TestInstance::new("BuildingBlocks_StaticVariableArray")
                        .value("name", TestValue::text(name.clone()))
                        .value("value", TestValue::Array(vec![TestValue::Struct(entry)])),
                )
            })
            .collect(),
    )
}

/// Returns the `modifiers` field of a clone: the modifiers apply to the widget it instantiates.
#[must_use]
pub fn clone_modifiers(modifiers: &[TestModifier]) -> TestValue {
    TestValue::Array(
        modifiers
            .iter()
            .map(|modifier| {
                TestValue::Struct(
                    TestInstance::new("BuildingBlocks_FieldModifierPair")
                        .value("target", TestValue::Null)
                        .value("modifier", modifier.value()),
                )
            })
            .collect(),
    )
}

fn vec3_of(x: f32, y: f32, z: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Vec3")
            .value("x", TestValue::Single(x))
            .value("y", TestValue::Single(y))
            .value("z", TestValue::Single(z)),
    )
}

impl TestWidget {
    /// Sets the pivot, the point of the widget that its anchor holds and that it turns and scales
    /// about, as shares of its own size.
    #[must_use]
    pub fn pivot(self, x: f32, y: f32) -> Self {
        self.with("pivot", vec3(x, y))
    }

    /// Sets the anchor, the point of the parent that the pivot sits on, as shares of the parent.
    #[must_use]
    pub fn anchor(self, x: f32, y: f32) -> Self {
        self.with("anchor", vec3(x, y))
    }

    /// Turns the widget in the plane of the screen by `degrees`, clockwise.
    #[must_use]
    pub fn rotate(self, degrees: f32) -> Self {
        self.orientation(0.0, 0.0, degrees)
    }

    /// Sets the orientation about the x, y and z axes in degrees.
    #[must_use]
    pub fn orientation(self, x: f32, y: f32, z: f32) -> Self {
        self.with("orientation", vec3_of(x, y, z))
    }

    /// Sets the scale along x and y.
    #[must_use]
    pub const fn scale(self, x: f32, y: f32) -> Self {
        self.set_scale([x, y])
    }

    /// Rounds the four corners of the box by `radius`.
    #[must_use]
    pub const fn rounded(self, radius: f32) -> Self {
        self.set_corner(radius, false)
    }

    /// Cuts the four corners of the box by `radius`.
    #[must_use]
    pub const fn chamfered(self, radius: f32) -> Self {
        self.set_corner(radius, true)
    }

    /// Gives the background a colour for one of its corners, `topLeftColor`, `topRightColor`,
    /// `bottomLeftColor` or `bottomRightCColor` as the data spells them, which makes it a gradient.
    #[must_use]
    pub const fn gradient(self, field: &'static str, corner: TestColour) -> Self {
        self.set_gradient(field, corner)
    }

    /// Sets the overflow, `Visible`, `Clip`, `ClipFade` or `Fade`.
    #[must_use]
    pub fn overflow(self, mode: &str) -> Self {
        self.with(
            "overflow",
            TestValue::Struct(
                TestInstance::new("BuildingBlocks_Overflow")
                    .value("overflow", TestValue::text(mode)),
            ),
        )
    }

    /// Enables a minimum width.
    #[must_use]
    pub fn min_width(self, dim: TestDim) -> Self {
        self.set_bound(0, dim)
    }

    /// Enables a minimum height.
    #[must_use]
    pub fn min_height(self, dim: TestDim) -> Self {
        self.set_bound(1, dim)
    }

    /// Enables a maximum width.
    #[must_use]
    pub fn max_width(self, dim: TestDim) -> Self {
        self.set_bound(2, dim)
    }

    /// Enables a maximum height.
    #[must_use]
    pub fn max_height(self, dim: TestDim) -> Self {
        self.set_bound(3, dim)
    }

    /// Sets the share by which the widget gives up space in an overfull flex container.
    #[must_use]
    pub const fn shrink(self, shrink: f32) -> Self {
        self.set_shrink(shrink)
    }

    /// Sets how a flex container wraps: `NoWrap`, `Wrap`, `WrapReverse` or `NoWrapInfinite`.
    #[must_use]
    pub fn wrap(self, wrap: &str) -> Self {
        self.set_flex_part(1, wrap)
    }

    /// Sets how a flex container places its lines across: `Start`, `Center`, `End`, `Stretch`, ...
    #[must_use]
    pub fn lines(self, justification: &str) -> Self {
        self.set_flex_part(4, justification)
    }

    /// Stacks the children along `direction`, `Horizontal` or `Vertical`, with `spacing` between.
    #[must_use]
    pub fn stack(self, direction: &str, spacing: f32) -> Self {
        self.set_policy(
            TestInstance::new("BuildingBlocks_StackContainer")
                .value("direction", TestValue::text(direction))
                .value("spacing", TestValue::Single(spacing)),
        )
    }

    /// Packs the children into a grid that fills along `direction` with `gutter` between cells.
    #[must_use]
    pub fn grid(self, direction: &str, gutter: f32) -> Self {
        self.set_policy(
            TestInstance::new("BuildingBlocks_GridPackContainer")
                .value("direction", TestValue::text(direction))
                .value("gutter", TestValue::Single(gutter)),
        )
    }

    /// Makes the widget a spacer container, which pushes its children apart physically.
    #[must_use]
    pub fn spacer(self, spacing: f32) -> Self {
        self.set_policy(
            TestInstance::new("BuildingBlocks_SpacerContainer")
                .value("spacing", TestValue::Single(spacing)),
        )
    }

    /// Creates a `BuildingBlocks_WidgetClone` of the widget `target`, a template of the library
    /// or any widget of the canvas.
    #[must_use]
    pub fn clone_of(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new("BuildingBlocks_WidgetClone", name).with("target", TestValue::pointer(target))
    }

    /// Creates a `BuildingBlocks_WidgetList` that repeats the widget `target` once per element of
    /// the array variable `array`.
    #[must_use]
    pub fn list(
        name: impl Into<String>,
        target: impl Into<String>,
        array: impl Into<String>,
    ) -> Self {
        Self::new("BuildingBlocks_WidgetList", name)
            .with("target", TestValue::pointer(target))
            .with("arrayVariable", TestValue::text(array))
    }

    /// Makes a canvas widget draw its canvas at the canvas's own size scaled to the widget
    /// (`Scale`) instead of laid out in the widget (`Size`).
    #[must_use]
    pub fn sized_by(self, method: &str) -> Self {
        self.with("sizingMethod", TestValue::text(method))
    }
}
