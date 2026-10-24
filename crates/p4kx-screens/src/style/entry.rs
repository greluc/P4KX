// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{Guid, StructValue, Value};

use crate::colour::ColorSpec;
use crate::reader::{as_struct, child, flag, float, items, reference, text};
use crate::style::{PictureModifiers, TextModifiers};

const PREFIX: &str = "BuildingBlocks_StyleSelectorCondition";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Condition {
    Tag(Guid),
    AnyOfTag(Vec<Guid>),
    AllOfTag(Vec<Guid>),
    NotTag(Guid),
    Type(String),
    Parent(Vec<Self>),
    Ancestor(Vec<Self>),
    AllOf(Vec<Self>),
    AnyOf(Vec<Self>),
    Unsupported(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Modifiers {
    pub(crate) background: Option<ColorSpec>,
    pub(crate) fill: Option<ColorSpec>,
    pub(crate) stroke: Option<ColorSpec>,
    pub(crate) border_colours: [Option<ColorSpec>; 4],
    pub(crate) border_widths: [Option<f32>; 4],
    pub(crate) enable_background: Option<bool>,
    pub(crate) active: Option<bool>,
    pub(crate) alpha: Option<f32>,
    pub(crate) text: TextModifiers,
    pub(crate) picture: PictureModifiers,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) lists: Vec<Vec<Condition>>,
    pub(crate) modifiers: Modifiers,
    pub(crate) unsupported: Vec<String>,
}

pub(crate) fn parse_entries(values: &[Value<'_>]) -> Vec<Entry> {
    values
        .iter()
        .filter_map(as_struct)
        .map(parse_entry)
        .filter(|entry| entry.modifiers != Modifiers::default())
        .collect()
}

fn parse_entry(entry: &StructValue<'_>) -> Entry {
    let lists: Vec<Vec<Condition>> = items(entry, "conditionsList")
        .iter()
        .filter_map(as_struct)
        .map(|list| conditions(items(list, "conditions")))
        .collect();
    let mut unsupported = Vec::new();
    for condition in lists.iter().flatten() {
        collect_unsupported(condition, &mut unsupported);
    }
    Entry {
        name: text(entry, "name").unwrap_or_default().to_owned(),
        lists,
        modifiers: modifiers(items(entry, "modifiers")),
        unsupported,
    }
}

fn collect_unsupported(condition: &Condition, out: &mut Vec<String>) {
    match condition {
        Condition::Unsupported(name) => {
            if !out.contains(name) {
                out.push(name.clone());
            }
        }
        Condition::Parent(inner)
        | Condition::Ancestor(inner)
        | Condition::AllOf(inner)
        | Condition::AnyOf(inner) => {
            for condition in inner {
                collect_unsupported(condition, out);
            }
        }
        _ => {}
    }
}

const MAX_CONDITION_DEPTH: usize = 8;
const MAX_LINK_DEPTH: usize = 4;

#[derive(Clone, Copy)]
struct Depth {
    all: usize,
    links: usize,
}

fn conditions(values: &[Value<'_>]) -> Vec<Condition> {
    conditions_at(values, Depth { all: 0, links: 0 })
}

fn conditions_at(values: &[Value<'_>], depth: Depth) -> Vec<Condition> {
    values
        .iter()
        .filter_map(as_struct)
        .map(|structure| condition(structure, depth))
        .collect()
}

fn guid_of(structure: &StructValue<'_>, name: &str) -> Option<Guid> {
    reference(structure, name).map(p4kx_dataforge::Reference::guid)
}

fn guids_of(structure: &StructValue<'_>, name: &str) -> Vec<Guid> {
    items(structure, name)
        .iter()
        .filter_map(|value| match value {
            Value::Reference(reference) => Some(reference.guid()),
            _ => None,
        })
        .collect()
}

fn condition(structure: &StructValue<'_>, depth: Depth) -> Condition {
    let kind = structure
        .type_name()
        .strip_prefix(PREFIX)
        .unwrap_or_else(|| structure.type_name());
    let link = matches!(kind, "Parent" | "Ancestor");
    if depth.all >= MAX_CONDITION_DEPTH || (link && depth.links >= MAX_LINK_DEPTH) {
        return Condition::Unsupported(format!("{kind} nested too deep"));
    }
    let deeper = Depth {
        all: depth.all + 1,
        links: depth.links + usize::from(link),
    };
    let nested = || conditions_at(items(structure, "conditions"), deeper);
    match kind {
        "Tag" => guid_of(structure, "tag")
            .map_or_else(|| Condition::Unsupported("Tag".to_owned()), Condition::Tag),
        "NotTag" => guid_of(structure, "tag").map_or_else(
            || Condition::Unsupported("NotTag".to_owned()),
            Condition::NotTag,
        ),
        "AnyOfTag" => Condition::AnyOfTag(guids_of(structure, "tags")),
        "AllOfTag" => Condition::AllOfTag(guids_of(structure, "tags")),
        "Type" => Condition::Type(text(structure, "type").unwrap_or_default().to_owned()),
        "Parent" => Condition::Parent(nested()),
        "Ancestor" => Condition::Ancestor(nested()),
        "AllOfCondition" => Condition::AllOf(nested()),
        "AnyOfCondition" => Condition::AnyOf(nested()),
        other => Condition::Unsupported(other.to_owned()),
    }
}

const BORDER_SIDES: [&str; 4] = ["Top", "Right", "Bottom", "Left"];

fn modifiers(values: &[Value<'_>]) -> Modifiers {
    let mut found = Modifiers::default();
    for modifier in values.iter().filter_map(as_struct) {
        if matches!(
            modifier.type_name(),
            "BuildingBlocks_FieldModifierRecordRef" | "BuildingBlocks_FieldModifierEnumerated"
        ) {
            if let Some(inner) = child(modifier, "field") {
                found.text.typed(inner);
            }
            continue;
        }
        let Some(field) = text(modifier, "field") else {
            continue;
        };
        match modifier.type_name() {
            "BuildingBlocks_FieldModifierColor" => {
                let colour = modifier.field("color").and_then(ColorSpec::parse);
                colour_field(&mut found, field, colour);
            }
            "BuildingBlocks_FieldModifierBoolean" => {
                let value = flag(modifier, "value");
                match field {
                    "EnableBackground" => found.enable_background = value,
                    "IsActive" => found.active = value,
                    _ => {
                        found.text.boolean(field, value);
                        found.picture.boolean(field, value);
                    }
                }
            }
            "BuildingBlocks_FieldModifierNumber" => {
                number_field(&mut found, field, float(modifier, "value"));
            }
            "BuildingBlocks_FieldModifierString" => {
                found.picture.string(field, text(modifier, "value"));
            }
            "BuildingBlocks_FieldModifierLocalization" => {
                found.text.localised(field, text(modifier, "value"));
            }
            _ => {}
        }
    }
    found
}

fn colour_field(found: &mut Modifiers, field: &str, colour: Option<ColorSpec>) {
    match field {
        "BackgroundColor" => found.background = colour,
        "FillColor" => found.fill = colour,
        "StrokeColor" => found.stroke = colour,
        _ => {
            if let Some(side) = field.strip_prefix("BorderColor")
                && let Some(index) = BORDER_SIDES.iter().position(|name| *name == side)
                && let Some(slot) = found.border_colours.get_mut(index)
            {
                *slot = colour;
            }
        }
    }
}

fn number_field(found: &mut Modifiers, field: &str, value: Option<f32>) {
    if field == "Alpha" {
        found.alpha = value;
        return;
    }
    found.text.number(field, value);
    if let Some(side) = field
        .strip_prefix("Border")
        .and_then(|rest| rest.strip_suffix("Width"))
        && let Some(index) = BORDER_SIDES.iter().position(|name| *name == side)
        && let Some(slot) = found.border_widths.get_mut(index)
    {
        *slot = value;
    }
}

pub(crate) fn inline_entries(widget: &StructValue<'_>) -> Vec<Entry> {
    parse_entries(items(widget, "inlineStyles"))
}
