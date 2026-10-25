// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::rc::Rc;

use p4kx_dataforge::{Location, Step, StructValue, Value};

use crate::reader::{as_struct, items, reference, text};

const PARAMETER: &str = "BuildingBlocks_BindingsLocalizedComponentParameter";
const FIELD: &str = "BuildingBlocks_BindingsLocalizedField";
const INPUT: &str = "BuildingBlocks_ComponentParameterInputLocalization";

/// The component parameter that fills the text of a widget, with the text it has when the
/// canvas that holds the component names no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextBinding {
    pub(crate) parameter: String,
    pub(crate) default: Option<String>,
}

/// The localised values a `WidgetCanvas` hands to the component parameters of its canvas.
pub(crate) type ParamValues = Rc<[(String, String)]>;

fn index_at(location: Option<&Location<'_>>, field: &str) -> Option<usize> {
    match location.map(Location::steps) {
        Some([Step::Field(name), Step::Element(index)]) if *name == field => {
            usize::try_from(*index).ok()
        }
        _ => None,
    }
}

/// Reads which widget texts a canvas binds to component parameters; the widgets of the library
/// follow the `scene_len` widgets of the scene.
pub(crate) fn text_bindings(
    root: &StructValue<'_>,
    scene_len: usize,
) -> HashMap<usize, TextBinding> {
    let operations = items(root, "operations");
    let parameters: HashMap<usize, TextBinding> = operations
        .iter()
        .enumerate()
        .filter_map(|(at, operation)| {
            let operation = as_struct(operation).filter(|op| op.type_name() == PARAMETER)?;
            Some((
                at,
                TextBinding {
                    parameter: text(operation, "parameter")?.to_owned(),
                    default: text(operation, "defaultValue").map(str::to_owned),
                },
            ))
        })
        .collect();
    operations
        .iter()
        .filter_map(as_struct)
        .filter(|operation| {
            operation.type_name() == FIELD && text(operation, "field") == Some("Text")
        })
        .filter_map(|operation| {
            let location = reference(operation, "widget")?.location();
            let widget = index_at(location, "scene").or_else(|| {
                index_at(location, "library").and_then(|index| index.checked_add(scene_len))
            })?;
            let input = index_at(reference(operation, "input")?.location(), "operations")?;
            Some((widget, parameters.get(&input)?.clone()))
        })
        .collect()
}

/// Reads the localised parameter values a widget hands to its nested canvas.
pub(crate) fn param_values(widget: &StructValue<'_>) -> ParamValues {
    items(widget, "paramInputValues")
        .iter()
        .filter_map(|value| match value {
            Value::Struct(input) if input.type_name() == INPUT => Some((
                text(input, "parameter")?.to_owned(),
                text(input, "value")?.to_owned(),
            )),
            _ => None,
        })
        .collect()
}
