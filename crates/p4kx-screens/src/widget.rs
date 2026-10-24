// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{Guid, Step, StructValue, Value};

use crate::assets::WidgetAssets;
use crate::bindings::{ParamValues, param_values};
use crate::colour::ColorSpec;
use crate::flex::{Flex, parse_flex};
use crate::picture::Picture;
use crate::reader::{child, flag, float, int, items, reference, text};
use crate::style::{Entry, inline_entries};
use crate::text::TextSpec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Dim {
    Fixed(f32),
    Percent(f32),
    OfWidth(f32),
    OfHeight(f32),
    Auto,
}

impl Dim {
    fn parse(value: Option<&StructValue<'_>>) -> Self {
        let Some(value) = value else {
            return Self::Auto;
        };
        let amount = float(value, "value").unwrap_or(0.0);
        match text(value, "behavior") {
            Some("Percent") => Self::Percent(amount),
            Some("PercentOfX") => Self::OfWidth(amount),
            Some("PercentOfY") => Self::OfHeight(amount),
            Some("Auto") => Self::Auto,
            _ => Self::Fixed(amount),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Edges {
    pub(crate) top: f32,
    pub(crate) right: f32,
    pub(crate) bottom: f32,
    pub(crate) left: f32,
}

impl Edges {
    fn parse(value: Option<&StructValue<'_>>) -> Self {
        let Some(value) = value else {
            return Self::default();
        };
        let edge = |name| float(value, name).unwrap_or(0.0);
        Self {
            top: edge("top"),
            right: edge("right"),
            bottom: edge("bottom"),
            left: edge("left"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Parent {
    Root,
    Widget(usize),
    Library,
}

#[derive(Debug, Clone)]
pub(crate) struct BorderSide {
    pub(crate) width: f32,
    pub(crate) color: Option<ColorSpec>,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the flags mirror the widget fields of the Record one to one"
)]
pub(crate) struct Widget {
    pub(crate) type_name: String,
    pub(crate) name: String,
    pub(crate) parent: Parent,
    pub(crate) active: bool,
    pub(crate) in_flow: bool,
    pub(crate) alpha: f32,
    pub(crate) inherits_alpha: bool,
    pub(crate) offset: [f32; 2],
    pub(crate) size: [Dim; 2],
    pub(crate) padding: Edges,
    pub(crate) margin: Edges,
    pub(crate) background: Option<ColorSpec>,
    pub(crate) background_enabled: bool,
    pub(crate) border: [BorderSide; 4],
    pub(crate) tags: Vec<Guid>,
    pub(crate) inline: Vec<Entry>,
    pub(crate) flex: Option<Flex>,
    pub(crate) other_layout: Option<String>,
    pub(crate) grow: f32,
    pub(crate) order: i32,
    pub(crate) layer: i32,
    pub(crate) rotated: bool,
    pub(crate) canvas: Option<Guid>,
    pub(crate) canvas_dangling: bool,
    pub(crate) shape: ShapeStyle,
    pub(crate) text: Option<TextSpec>,
    pub(crate) picture: Picture,
    pub(crate) params: ParamValues,
    pub(crate) assets: WidgetAssets,
}

#[derive(Debug, Clone)]
pub(crate) struct ShapeStyle {
    pub(crate) fill: Option<ColorSpec>,
    pub(crate) do_fill: bool,
    pub(crate) stroke: Option<ColorSpec>,
    pub(crate) do_stroke: bool,
    pub(crate) stroke_width: f32,
    pub(crate) sides: u32,
    pub(crate) start_angle: f32,
}

impl ShapeStyle {
    fn parse(structure: &StructValue<'_>) -> Self {
        let colour = |name| structure.field(name).and_then(ColorSpec::parse);
        Self {
            fill: colour("fillColor"),
            do_fill: flag(structure, "doFill").unwrap_or(true),
            stroke: colour("strokeColor"),
            do_stroke: flag(structure, "doStroke").unwrap_or(false),
            stroke_width: float(structure, "strokeWidth").unwrap_or(1.0),
            sides: int(structure, "sides")
                .and_then(|sides| u32::try_from(sides).ok())
                .unwrap_or(0),
            start_angle: float(structure, "startAngle").unwrap_or(0.0),
        }
    }
}

impl Edges {
    fn scaled(self, unit: f32) -> Self {
        Self {
            top: self.top * unit,
            right: self.right * unit,
            bottom: self.bottom * unit,
            left: self.left * unit,
        }
    }
}

impl Dim {
    fn scaled(self, unit: f32) -> Self {
        match self {
            Self::Fixed(value) => Self::Fixed(value * unit),
            other => other,
        }
    }
}

impl Widget {
    pub(crate) fn scale_units(&mut self, unit: f32) {
        self.offset = self.offset.map(|offset| offset * unit);
        self.size = self.size.map(|dim| dim.scaled(unit));
        self.padding = self.padding.scaled(unit);
        self.margin = self.margin.scaled(unit);
        for side in &mut self.border {
            side.width *= unit;
        }
        if let Some(flex) = &mut self.flex {
            flex.gap *= unit;
        }
        self.shape.stroke_width *= unit;
    }

    pub(crate) fn parse(structure: &StructValue<'_>, canvas: Guid) -> Self {
        let sizing = child(structure, "sizing");
        let dim = |name| Dim::parse(sizing.and_then(|sizing| child(sizing, name)));
        let position = vec2(structure, "position");
        let shift = vec2(structure, "positionOffset");
        let layout = child(structure, "layoutPolicy");
        let (flex, other_layout) = layout.map_or((None, None), parse_flex);
        Self {
            type_name: structure.type_name().to_owned(),
            name: text(structure, "name").unwrap_or_default().to_owned(),
            parent: parent_of(structure, canvas),
            active: flag(structure, "isActive").unwrap_or(true),
            in_flow: flag(structure, "affectsLayout").unwrap_or(true),
            alpha: float(structure, "alpha").unwrap_or(1.0),
            inherits_alpha: flag(structure, "inheritsAlpha").unwrap_or(true),
            offset: [position[0] + shift[0], position[1] + shift[1]],
            size: [dim("width"), dim("height")],
            padding: Edges::parse(child(structure, "padding")),
            margin: Edges::parse(child(structure, "margin")),
            background: child(structure, "background")
                .and_then(|background| background.field("color"))
                .and_then(ColorSpec::parse),
            background_enabled: child(structure, "background")
                .and_then(|background| flag(background, "enable"))
                .unwrap_or(false),
            border: borders(structure),
            tags: items(structure, "styleTags")
                .iter()
                .filter_map(|tag| match tag {
                    Value::Reference(reference) => Some(reference.guid()),
                    _ => None,
                })
                .collect(),
            inline: inline_entries(structure),
            flex,
            other_layout,
            grow: child(structure, "layoutPolicyItem")
                .and_then(|item| float(item, "growProportion"))
                .unwrap_or(0.0),
            order: child(structure, "layoutItemCommon")
                .and_then(|common| int(common, "order"))
                .unwrap_or(0),
            layer: int(structure, "layer").unwrap_or(0),
            rotated: rotated(structure),
            canvas: reference(structure, "canvas").map(p4kx_dataforge::Reference::guid),
            canvas_dangling: matches!(structure.field("canvas"), Some(Value::Error(_))),
            shape: ShapeStyle::parse(structure),
            text: TextSpec::parse(structure),
            picture: Picture::parse(structure),
            params: param_values(structure),
            assets: WidgetAssets::scan(structure),
        }
    }
}

fn vec2(structure: &StructValue<'_>, name: &str) -> [f32; 2] {
    child(structure, name).map_or([0.0; 2], |vector| {
        [
            float(vector, "x").unwrap_or(0.0),
            float(vector, "y").unwrap_or(0.0),
        ]
    })
}

fn rotated(structure: &StructValue<'_>) -> bool {
    ["orientation", "orientationOffset"].iter().any(|name| {
        child(structure, name).is_some_and(|angles| {
            ["x", "y", "z"]
                .iter()
                .any(|axis| float(angles, axis).is_some_and(|angle| angle != 0.0))
        })
    })
}

fn parent_of(structure: &StructValue<'_>, canvas: Guid) -> Parent {
    let Some(parent) = reference(structure, "parent") else {
        return Parent::Root;
    };
    if parent.guid() != canvas {
        return Parent::Root;
    }
    match parent.location().map(p4kx_dataforge::Location::steps) {
        Some([Step::Field("scene"), Step::Element(index)]) => {
            usize::try_from(*index).map_or(Parent::Root, Parent::Widget)
        }
        Some([Step::Field("library"), Step::Element(_)]) => Parent::Library,
        _ => Parent::Root,
    }
}

fn borders(structure: &StructValue<'_>) -> [BorderSide; 4] {
    let border = child(structure, "border");
    let side = |name| {
        let side = border.and_then(|border| child(border, name));
        BorderSide {
            width: side.and_then(|side| float(side, "width")).unwrap_or(0.0),
            color: side
                .and_then(|side| side.field("color"))
                .and_then(ColorSpec::parse),
        }
    };
    [side("top"), side("right"), side("bottom"), side("left")]
}
