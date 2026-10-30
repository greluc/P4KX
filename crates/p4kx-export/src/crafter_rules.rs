// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The crafting modifiers and the blueprint category rules of a crafter.
//!
//! Modifier types, modifier conditions and queue availabilities are polymorphic structs. They are
//! recognised by their struct name; a struct that is not known is kept with its name and no
//! values, and every deviation is reported as an issue code.

use p4kx_extraction::{StructValue, Value};

use crate::contract_crafter::{CategoryRule, CrafterModifier};
use crate::crafter_fields::{Issues, array};
use crate::record_fields::{number, references};

const TIME_TYPE: &str = "CraftingModifierType_Time";
const CATEGORY_CONDITION: &str = "CraftingModifierCondition_BlueprintCategory";
const WHITELIST: &str = "BlueprintCategoryAvailability_Whitelist";

const UNREADABLE_MODIFIER: &str = "unreadable_modifier";
const UNREADABLE_MODIFIERS: &str = "unreadable_modifiers";
const UNREADABLE_PROCESS_TYPES: &str = "unreadable_process_types";
const MISSING_MODIFIER_TYPE: &str = "missing_modifier_type";
const UNKNOWN_MODIFIER_TYPE: &str = "unknown_modifier_type";
const UNREADABLE_TIME_MULTIPLIER: &str = "unreadable_time_multiplier";
const UNKNOWN_MODIFIER_CONDITION: &str = "unknown_modifier_condition";
const UNKNOWN_CATEGORY_AVAILABILITY: &str = "unknown_category_availability";
const UNREADABLE_CATEGORY: &str = "unreadable_category";

/// What a polymorphic struct field holds.
enum Pointer<'t, 'f> {
    /// The field is absent or a null pointer.
    Null,
    Struct(&'t StructValue<'f>),
    /// Anything else, e.g. a value that could not be read.
    Unreadable,
}

fn pointer<'t, 'f>(parent: &'t StructValue<'f>, name: &str) -> Pointer<'t, 'f> {
    match parent.field(name) {
        None | Some(Value::Null) => Pointer::Null,
        Some(Value::Struct(value)) => Pointer::Struct(value),
        Some(_) => Pointer::Unreadable,
    }
}

/// Returns the modifiers the `modifiers` array of the crafter component `params` holds, in stored
/// order; an element that is no struct is kept as an `unknown` modifier.
pub(crate) fn modifiers<'f>(
    params: &StructValue<'f>,
    issues: &mut Issues,
) -> Vec<CrafterModifier<'f>> {
    array(Some(params), "modifiers", UNREADABLE_MODIFIERS, issues)
        .iter()
        .map(|element| read_modifier(element, issues))
        .collect()
}

fn read_modifier<'f>(element: &Value<'f>, issues: &mut Issues) -> CrafterModifier<'f> {
    let Value::Struct(modifier) = element else {
        issues.insert(UNREADABLE_MODIFIER);
        return CrafterModifier {
            r#type: "unknown",
            struct_name: None,
            time_multiplier: None,
            condition: no_rule(),
        };
    };
    let (r#type, struct_name, time_multiplier) = match pointer(modifier, "modifierType") {
        Pointer::Null => {
            issues.insert(MISSING_MODIFIER_TYPE);
            ("none", None, None)
        }
        Pointer::Struct(value) if value.type_name() == TIME_TYPE => {
            let multiplier = number(Some(value), "timeMultiplier");
            if multiplier.is_none() {
                issues.insert(UNREADABLE_TIME_MULTIPLIER);
            }
            ("time", Some(value.type_name()), multiplier)
        }
        Pointer::Struct(value) => {
            issues.insert(UNKNOWN_MODIFIER_TYPE);
            ("unknown", Some(value.type_name()), None)
        }
        Pointer::Unreadable => {
            issues.insert(UNREADABLE_MODIFIER);
            ("unknown", None, None)
        }
    };
    CrafterModifier {
        r#type,
        struct_name,
        time_multiplier,
        condition: rule(
            modifier,
            "modifierCondition",
            (CATEGORY_CONDITION, "blueprintCategory"),
            UNKNOWN_MODIFIER_CONDITION,
            issues,
        ),
    }
}

/// Returns the blueprint categories the queue `queue` accepts.
pub(crate) fn availability<'f>(queue: &StructValue<'f>, issues: &mut Issues) -> CategoryRule<'f> {
    rule(
        queue,
        "blueprintCategoryAvailability",
        (WHITELIST, "whitelist"),
        UNKNOWN_CATEGORY_AVAILABILITY,
        issues,
    )
}

/// Returns the booleans of `processTypeAvailability`. The list is empty, with an issue, when an
/// element is no boolean, because a lost position would shift every later process type.
pub(crate) fn process_types(queue: &StructValue<'_>, issues: &mut Issues) -> Vec<bool> {
    let flags: Option<Vec<bool>> = array(
        Some(queue),
        "processTypeAvailability",
        UNREADABLE_PROCESS_TYPES,
        issues,
    )
    .iter()
    .map(|flag| match flag {
        Value::Bool(flag) => Some(*flag),
        _ => None,
    })
    .collect();
    flags.unwrap_or_else(|| {
        issues.insert(UNREADABLE_PROCESS_TYPES);
        Vec::new()
    })
}

pub(crate) const fn no_rule<'f>() -> CategoryRule<'f> {
    CategoryRule {
        kind: "none",
        struct_name: None,
        blueprint_category_guids: Vec::new(),
    }
}

/// Reads the polymorphic struct in the field `name` of `parent`. `known` is the struct this
/// export understands and the `kind` it gets; any other struct is `unknown` and reported with
/// `unknown_issue`.
fn rule<'f>(
    parent: &StructValue<'f>,
    name: &str,
    known: (&str, &'static str),
    unknown_issue: &'static str,
    issues: &mut Issues,
) -> CategoryRule<'f> {
    match pointer(parent, name) {
        Pointer::Null => no_rule(),
        Pointer::Struct(value) if value.type_name() == known.0 => {
            let guids = references(Some(value), "categories");
            if guids.len() != array(Some(value), "categories", UNREADABLE_CATEGORY, issues).len() {
                issues.insert(UNREADABLE_CATEGORY);
            }
            CategoryRule {
                kind: known.1,
                struct_name: Some(value.type_name()),
                blueprint_category_guids: guids,
            }
        }
        Pointer::Struct(value) => {
            issues.insert(unknown_issue);
            CategoryRule {
                kind: "unknown",
                struct_name: Some(value.type_name()),
                blueprint_category_guids: Vec::new(),
            }
        }
        Pointer::Unreadable => {
            issues.insert(unknown_issue);
            CategoryRule {
                kind: "unknown",
                struct_name: None,
                blueprint_category_guids: Vec::new(),
            }
        }
    }
}
