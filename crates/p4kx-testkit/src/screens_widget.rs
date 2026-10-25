// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

pub use crate::screens_value::{TestColour, TestDim};
use crate::{TestInstance, TestValue};

fn trbl([top, right, bottom, left]: [f32; 4]) -> TestValue {
    TestValue::Struct(
        TestInstance::new("BuildingBlocks_TRBL")
            .value("top", TestValue::Single(top))
            .value("right", TestValue::Single(right))
            .value("bottom", TestValue::Single(bottom))
            .value("left", TestValue::Single(left)),
    )
}

pub(crate) fn vec3(x: f32, y: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Vec3")
            .value("x", TestValue::Single(x))
            .value("y", TestValue::Single(y))
            .value("z", TestValue::Single(0.0)),
    )
}

/// One widget of a synthetic canvas, a `BuildingBlocks_DisplayWidget` or a derived type.
#[derive(Debug, Clone)]
pub struct TestWidget {
    pub(crate) kind: &'static str,
    pub(crate) name: String,
    parent: Option<String>,
    size: [TestDim; 2],
    offset: [f32; 2],
    margin: [f32; 4],
    padding: [f32; 4],
    background: Option<TestColour>,
    border: Option<(f32, TestColour)>,
    flex: Option<[String; 5]>,
    gap: f32,
    grow: f32,
    shrink: Option<f32>,
    active: bool,
    in_flow: bool,
    alpha: f32,
    tags: Vec<u128>,
    bounds: [Option<TestDim>; 4],
    policy: Option<TestInstance>,
    scale: [f32; 2],
    corner: Option<(f32, bool)>,
    gradient: Option<(&'static str, TestColour)>,
    extra: Vec<(&'static str, TestValue)>,
}

impl TestWidget {
    /// Creates a widget of the struct `kind`, e.g. `BuildingBlocks_WidgetCard`, filling its
    /// parent.
    #[must_use]
    pub fn new(kind: &'static str, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            parent: None,
            size: [TestDim::Percent(1.0); 2],
            offset: [0.0; 2],
            margin: [0.0; 4],
            padding: [0.0; 4],
            background: None,
            border: None,
            flex: None,
            gap: 0.0,
            grow: 0.0,
            shrink: None,
            active: true,
            in_flow: true,
            alpha: 1.0,
            tags: Vec::new(),
            bounds: [None; 4],
            policy: None,
            scale: [1.0; 2],
            corner: None,
            gradient: None,
            extra: Vec::new(),
        }
    }

    pub(crate) const fn set_gradient(mut self, field: &'static str, corner: TestColour) -> Self {
        self.gradient = Some((field, corner));
        self
    }

    pub(crate) const fn set_corner(mut self, radius: f32, chamfer: bool) -> Self {
        self.corner = Some((radius, chamfer));
        self
    }

    pub(crate) const fn set_scale(mut self, scale: [f32; 2]) -> Self {
        self.scale = scale;
        self
    }

    pub(crate) fn set_policy(mut self, policy: TestInstance) -> Self {
        self.policy = Some(policy);
        self
    }

    pub(crate) fn set_bound(mut self, at: usize, dim: TestDim) -> Self {
        if let Some(slot) = self.bounds.get_mut(at) {
            *slot = Some(dim);
        }
        self
    }

    pub(crate) fn set_flex_part(mut self, at: usize, value: &str) -> Self {
        if let Some(slot) = self.flex.as_mut().and_then(|flex| flex.get_mut(at)) {
            value.clone_into(slot);
        }
        self
    }

    pub(crate) const fn set_shrink(mut self, shrink: f32) -> Self {
        self.shrink = Some(shrink);
        self
    }

    /// Creates a `BuildingBlocks_DisplayWidget`.
    #[must_use]
    pub fn display(name: impl Into<String>) -> Self {
        Self::new("BuildingBlocks_DisplayWidget", name)
    }

    /// Makes the widget a child of the widget `name`, which must come earlier in the canvas.
    #[must_use]
    pub fn parent(mut self, name: impl Into<String>) -> Self {
        self.parent = Some(name.into());
        self
    }

    /// Sets width and height.
    #[must_use]
    pub const fn size(mut self, width: TestDim, height: TestDim) -> Self {
        self.size = [width, height];
        self
    }

    /// Sets the position offset.
    #[must_use]
    pub const fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = [x, y];
        self
    }

    /// Sets the margin: top, right, bottom, left.
    #[must_use]
    pub const fn margin(mut self, margin: [f32; 4]) -> Self {
        self.margin = margin;
        self
    }

    /// Sets the padding: top, right, bottom, left.
    #[must_use]
    pub const fn padding(mut self, padding: [f32; 4]) -> Self {
        self.padding = padding;
        self
    }

    /// Enables the background in `colour`.
    #[must_use]
    pub const fn background(mut self, colour: TestColour) -> Self {
        self.background = Some(colour);
        self
    }

    /// Draws a border of `width` in `colour` on all four sides.
    #[must_use]
    pub const fn border(mut self, width: f32, colour: TestColour) -> Self {
        self.border = Some((width, colour));
        self
    }

    /// Lays the children out in a flex container with this `direction` (`Row`, `Column`, ...),
    /// `justification` along the axis, `alignment` across it and `gap` between them.
    #[must_use]
    pub fn flex(mut self, direction: &str, justification: &str, alignment: &str, gap: f32) -> Self {
        self.flex = Some([
            direction.to_owned(),
            "NoWrap".to_owned(),
            justification.to_owned(),
            alignment.to_owned(),
            "Start".to_owned(),
        ]);
        self.gap = gap;
        self
    }

    /// Sets the share of free space the widget takes in a flex container.
    #[must_use]
    pub const fn grow(mut self, grow: f32) -> Self {
        self.grow = grow;
        self
    }

    /// Hides the widget and its subtree.
    #[must_use]
    pub const fn hidden(mut self) -> Self {
        self.active = false;
        self
    }

    /// Takes the widget out of the layout flow.
    #[must_use]
    pub const fn out_of_flow(mut self) -> Self {
        self.in_flow = false;
        self
    }

    /// Sets the widget's own alpha.
    #[must_use]
    pub const fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// Gives the widget the style tag `Tag` Record `guid`.
    #[must_use]
    pub fn tag(mut self, guid: u128) -> Self {
        self.tags.push(guid);
        self
    }

    /// Sets a field of the derived struct, e.g. `canvas` of a canvas widget.
    #[must_use]
    pub fn with(mut self, field: &'static str, value: TestValue) -> Self {
        self.extra.push((field, value));
        self
    }

    pub(crate) fn value(&self) -> TestValue {
        let mut instance = TestInstance::new(self.kind)
            .named(self.name.clone())
            .value("name", TestValue::text(self.name.clone()))
            .value("isActive", TestValue::Bool(self.active))
            .value("affectsLayout", TestValue::Bool(self.in_flow))
            .value("inheritsAlpha", TestValue::Bool(true))
            .value("alpha", TestValue::Single(self.alpha))
            .value(
                "styleTags",
                TestValue::Array(
                    self.tags
                        .iter()
                        .map(|tag| TestValue::Reference(*tag))
                        .collect(),
                ),
            )
            .value("positionOffset", vec3(self.offset[0], self.offset[1]))
            .value("scale", vec3(self.scale[0], self.scale[1]))
            .value("sizing", self.sizing())
            .value("padding", trbl(self.padding))
            .value("margin", trbl(self.margin));
        if let Some(parent) = &self.parent {
            instance = instance.value("parent", TestValue::pointer(parent.clone()));
        }
        if let Some(colour) = self.background {
            let mut background = TestInstance::new("BuildingBlocks_Background")
                .value("enable", TestValue::Bool(true))
                .value("color", colour.value());
            if let Some((field, corner)) = self.gradient {
                background = background.value(field, corner.value());
            }
            instance = instance.value("background", TestValue::Struct(background));
        }
        instance = self.with_border(instance);
        instance = self.with_layout(instance);
        for (field, value) in &self.extra {
            instance = instance.value(*field, value.clone());
        }
        TestValue::Struct(instance)
    }

    fn sizing(&self) -> TestValue {
        let mut sizing = TestInstance::new("BuildingBlocks_Size")
            .value("width", self.size[0].value())
            .value("height", self.size[1].value());
        let names = [
            ("enableMinWidth", "minWidth"),
            ("enableMinHeight", "minHeight"),
            ("enableMaxWidth", "maxWidth"),
            ("enableMaxHeight", "maxHeight"),
        ];
        for (bound, (enable, field)) in self.bounds.iter().zip(names) {
            if let Some(dim) = bound {
                sizing = sizing
                    .value(enable, TestValue::Bool(true))
                    .value(field, dim.value());
            }
        }
        TestValue::Struct(sizing)
    }

    fn with_border(&self, instance: TestInstance) -> TestInstance {
        if self.border.is_none() && self.corner.is_none() {
            return instance;
        }
        let mut border = TestInstance::new("BuildingBlocks_Border");
        if let Some((width, colour)) = self.border {
            let side = || {
                TestValue::Struct(
                    TestInstance::new("BuildingBlocks_BorderSide")
                        .value("width", TestValue::Single(width))
                        .value("color", colour.value()),
                )
            };
            border = border
                .value("top", side())
                .value("right", side())
                .value("bottom", side())
                .value("left", side());
        }
        if let Some((radius, chamfer)) = self.corner {
            for name in [
                "topLeftRadius",
                "topRightRadius",
                "bottomRightRadius",
                "bottomLeftRadius",
            ] {
                border = border.value(
                    name,
                    TestValue::Struct(
                        TestInstance::new("BuildingBlocks_BorderRadiusCorner")
                            .value("chamfer", TestValue::Bool(chamfer))
                            .value("radius", TestDim::Fixed(radius).value()),
                    ),
                );
            }
        }
        instance.value("border", TestValue::Struct(border))
    }

    fn with_layout(&self, mut instance: TestInstance) -> TestInstance {
        if let Some([direction, wrap, justification, alignment, cross]) = &self.flex {
            instance = instance.value(
                "layoutPolicy",
                TestValue::Struct(
                    TestInstance::new("BuildingBlocks_FlexContainer")
                        .value("direction", TestValue::text(direction.clone()))
                        .value("wrap", TestValue::text(wrap.clone()))
                        .value("axisJustification", TestValue::text(justification.clone()))
                        .value("crossAxisJustification", TestValue::text(cross.clone()))
                        .value("itemAlignment", TestValue::text(alignment.clone()))
                        .value("columnSpacing", TestValue::Single(self.gap))
                        .value("rowSpacing", TestValue::Single(self.gap)),
                ),
            );
        } else if let Some(policy) = &self.policy {
            instance = instance.value("layoutPolicy", TestValue::Struct(policy.clone()));
        }
        if self.grow > 0.0 || self.shrink.is_some() {
            instance = instance.value(
                "layoutPolicyItem",
                TestValue::Struct(
                    TestInstance::new("BuildingBlocks_FlexItem")
                        .value("growProportion", TestValue::Single(self.grow))
                        .value(
                            "shrinkProportion",
                            TestValue::Single(self.shrink.unwrap_or(0.0)),
                        ),
                ),
            );
        }
        instance
    }
}
