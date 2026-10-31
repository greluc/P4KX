// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `Requirement` of a loadout assortment or of one of its groups: decided where the Archive
//! decides it.

use p4kx_dataforge::{StructValue, Value};

use crate::read::has_items;

const TAGS: &str = "SLoadoutRequirementTags";
const ANY_OF: &str = "SLoadoutRequirementOR";
const INVERT: &str = "SLoadoutRequirementNOT";

/// Requirements nest at most three deep in LIVE (a NOT around an OR of tag sets); deeper ones are
/// undecided instead of recursing without bound.
const MAX_DEPTH: usize = 16;

/// What the Archive says about a Requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// It holds for every spawn: no Requirement, or one that restricts nothing.
    Met,
    /// It holds for no spawn.
    Unmet,
    /// It depends on tags that only the game knows at spawn time.
    Undecided,
}

impl Verdict {
    const fn not(self) -> Self {
        match self {
            Self::Met => Self::Unmet,
            Self::Unmet => Self::Met,
            Self::Undecided => Self::Undecided,
        }
    }
}

/// Evaluates the Requirement a field holds; no field or a null pointer is no Requirement.
///
/// A tag Requirement with neither required nor banned tags restricts nothing. Anything that names
/// a tag is [`Verdict::Undecided`]: the Archive does not hold the tags of the spawn. `OR` and
/// `NOT` combine the verdicts of their parts; an `OR` without parts, a `NOT` without a part and
/// a Requirement of a type not known here are undecided.
pub(crate) fn evaluate(requirement: Option<&Value<'_>>) -> Verdict {
    match requirement {
        None | Some(Value::Null) => Verdict::Met,
        Some(value) => evaluate_value(value, 0),
    }
}

fn evaluate_value(value: &Value<'_>, depth: usize) -> Verdict {
    match value {
        Value::Struct(structure) if depth < MAX_DEPTH => evaluate_struct(structure, depth),
        _ => Verdict::Undecided,
    }
}

fn evaluate_struct(structure: &StructValue<'_>, depth: usize) -> Verdict {
    match structure.type_name() {
        TAGS => {
            if has_items(structure, "RequiredTags") || has_items(structure, "BannedTags") {
                Verdict::Undecided
            } else {
                Verdict::Met
            }
        }
        ANY_OF => any_of(structure, depth),
        INVERT => structure
            .field("RequirementToInvert")
            .map_or(Verdict::Undecided, |inner| {
                evaluate_value(inner, depth + 1).not()
            }),
        _ => Verdict::Undecided,
    }
}

fn any_of(structure: &StructValue<'_>, depth: usize) -> Verdict {
    let Some(Value::Array(parts)) = structure.field("Requirements") else {
        return Verdict::Undecided;
    };
    if parts.is_empty() {
        return Verdict::Undecided;
    }
    let mut verdict = Verdict::Unmet;
    for part in parts {
        match evaluate_value(part, depth + 1) {
            Verdict::Met => return Verdict::Met,
            Verdict::Undecided => verdict = Verdict::Undecided,
            Verdict::Unmet => {}
        }
    }
    verdict
}
