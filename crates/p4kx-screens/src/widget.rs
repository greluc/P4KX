// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{Guid, Location, Reference, Step, StructValue, Value};

use crate::assets::WidgetAssets;
use crate::bindings::{ParamValues, param_values};
use crate::colour::ColorSpec;
use crate::flex::{Flex, parse_flex};
use crate::geometry::{Bounds, Dim, Edges, Overflow};
use crate::reader::{as_struct, child, flag, float, int, items, reference, text};
use crate::style::{Entry, Modifiers, inline_entries, modifiers};
use crate::text::TextSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Parent {
    Root,
    Widget(usize),
    Library(usize),
}

/// The widget a clone or a list repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Library(usize),
    Scene(usize),
}

/// A modifier a clone applies to the widget it instantiates, or to one widget of the subtree.
#[derive(Debug, Clone)]
pub(crate) struct CloneModifier {
    pub(crate) target: Option<Target>,
    pub(crate) modifiers: Modifiers,
}

#[derive(Debug, Clone)]
pub(crate) struct BorderSide {
    pub(crate) width: f32,
    pub(crate) color: Option<ColorSpec>,
}

/// How a canvas widget lays its canvas out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanvasSizing {
    Size,
    Scale,
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
    pub(crate) position: [f32; 2],
    pub(crate) position_offset: [f32; 2],
    pub(crate) size: [Dim; 2],
    pub(crate) bounds: Bounds,
    pub(crate) padding: Edges,
    pub(crate) margin: Edges,
    pub(crate) pivot: [f32; 2],
    pub(crate) anchor: [f32; 2],
    pub(crate) orientation: [f32; 3],
    pub(crate) orientation_offset: [f32; 3],
    pub(crate) scale: [f32; 2],
    pub(crate) overflow: Overflow,
    pub(crate) background: Option<ColorSpec>,
    pub(crate) background_enabled: bool,
    pub(crate) border: [BorderSide; 4],
    pub(crate) tags: Vec<Guid>,
    pub(crate) inline: Vec<Entry>,
    pub(crate) flex: Option<Flex>,
    pub(crate) other_layout: Option<String>,
    pub(crate) grow: f32,
    pub(crate) shrink: f32,
    pub(crate) order: i32,
    pub(crate) layer: i32,
    pub(crate) canvas: Option<Guid>,
    pub(crate) canvas_dangling: bool,
    pub(crate) canvas_sizing: CanvasSizing,
    pub(crate) shape: ShapeStyle,
    pub(crate) text: Option<TextSpec>,
    pub(crate) params: ParamValues,
    pub(crate) assets: WidgetAssets,
    pub(crate) target: Option<Target>,
    pub(crate) array_variable: Option<String>,
    pub(crate) clone_modifiers: Vec<CloneModifier>,
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

impl Widget {
    pub(crate) fn parse(structure: &StructValue<'_>, canvas: Guid) -> Self {
        let sizing = child(structure, "sizing");
        let dim = |name| Dim::parse(sizing.and_then(|sizing| child(sizing, name)));
        let layout = child(structure, "layoutPolicy");
        let (flex, other_layout) = layout.map_or((None, None), parse_flex);
        let item = child(structure, "layoutPolicyItem");
        Self {
            type_name: structure.type_name().to_owned(),
            name: text(structure, "name").unwrap_or_default().to_owned(),
            parent: parent_of(structure, canvas),
            active: flag(structure, "isActive").unwrap_or(true),
            in_flow: flag(structure, "affectsLayout").unwrap_or(true),
            alpha: float(structure, "alpha").unwrap_or(1.0),
            inherits_alpha: flag(structure, "inheritsAlpha").unwrap_or(true),
            position: vec2(structure, "position", 0.0),
            position_offset: vec2(structure, "positionOffset", 0.0),
            size: [dim("width"), dim("height")],
            bounds: Bounds::parse(sizing),
            padding: Edges::parse(child(structure, "padding")),
            margin: Edges::parse(child(structure, "margin")),
            pivot: vec2(structure, "pivot", 0.0),
            anchor: vec2(structure, "anchor", 0.0),
            orientation: vec3(structure, "orientation", 0.0),
            orientation_offset: vec3(structure, "orientationOffset", 0.0),
            scale: vec2(structure, "scale", 1.0),
            overflow: Overflow::parse(structure),
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
            grow: item
                .and_then(|item| float(item, "growProportion"))
                .unwrap_or(0.0),
            shrink: item
                .and_then(|item| float(item, "shrinkProportion"))
                .unwrap_or(0.0),
            order: child(structure, "layoutItemCommon")
                .and_then(|common| int(common, "order"))
                .unwrap_or(0),
            layer: int(structure, "layer").unwrap_or(0),
            canvas: reference(structure, "canvas").map(Reference::guid),
            canvas_dangling: matches!(structure.field("canvas"), Some(Value::Error(_))),
            canvas_sizing: if text(structure, "sizingMethod") == Some("Scale") {
                CanvasSizing::Scale
            } else {
                CanvasSizing::Size
            },
            shape: ShapeStyle::parse(structure),
            text: TextSpec::parse(structure),
            params: param_values(structure),
            assets: WidgetAssets::scan(structure),
            target: reference(structure, "target").and_then(|target| target_of(target, canvas)),
            array_variable: text(structure, "arrayVariable")
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
            clone_modifiers: clone_modifiers(structure, canvas),
        }
    }
}

fn vec2(structure: &StructValue<'_>, name: &str, fallback: f32) -> [f32; 2] {
    child(structure, name).map_or([fallback; 2], |vector| {
        [
            float(vector, "x").unwrap_or(fallback),
            float(vector, "y").unwrap_or(fallback),
        ]
    })
}

fn vec3(structure: &StructValue<'_>, name: &str, fallback: f32) -> [f32; 3] {
    child(structure, name).map_or([fallback; 3], |vector| {
        ["x", "y", "z"].map(|axis| float(vector, axis).unwrap_or(fallback))
    })
}

fn target_of(target: &Reference<'_>, canvas: Guid) -> Option<Target> {
    if target.guid() != canvas {
        return None;
    }
    at_index(target.location())
}

fn at_index(location: Option<&Location<'_>>) -> Option<Target> {
    match location.map(Location::steps) {
        Some([Step::Field("scene"), Step::Element(index)]) => {
            usize::try_from(*index).ok().map(Target::Scene)
        }
        Some([Step::Field("library"), Step::Element(index)]) => {
            usize::try_from(*index).ok().map(Target::Library)
        }
        _ => None,
    }
}

fn clone_modifiers(structure: &StructValue<'_>, canvas: Guid) -> Vec<CloneModifier> {
    items(structure, "modifiers")
        .iter()
        .filter_map(as_struct)
        .filter_map(|pair| {
            let modifier = child(pair, "modifier")?;
            Some(CloneModifier {
                target: reference(pair, "target").and_then(|target| target_of(target, canvas)),
                modifiers: modifiers(std::iter::once(modifier)),
            })
        })
        .collect()
}

fn parent_of(structure: &StructValue<'_>, canvas: Guid) -> Parent {
    let Some(parent) = reference(structure, "parent") else {
        return Parent::Root;
    };
    match target_of(parent, canvas) {
        Some(Target::Scene(index)) => Parent::Widget(index),
        Some(Target::Library(index)) => Parent::Library(index),
        None => Parent::Root,
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
