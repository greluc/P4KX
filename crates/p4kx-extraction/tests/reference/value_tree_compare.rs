// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Comparison of a Record P4KX built with the one a reference tool wrote.

#![expect(
    clippy::float_cmp,
    reason = "numbers are compared exactly at the precision of the field; a margin would hide a smallest-step change"
)]

use std::fmt::Write as _;

use super::tally::Compared;
use super::value_tree::{JsonNumber, Node, RecordDoc};

pub(crate) const WEAK_NULL: &str = "StarBreaker writes null for a weak pointer in a derived type";
pub(crate) const UNRESOLVED_NULL: &str =
    "StarBreaker writes null for a value P4KX reports as an error";
pub(crate) const LENIENT_BOOL: &str = "StarBreaker reads any non-zero bool byte as true";

const EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

#[expect(
    clippy::cast_possible_truncation,
    reason = "narrowing to single precision is the comparison"
)]
const fn narrow(number: f64) -> f32 {
    number as f32
}

#[expect(
    clippy::cast_precision_loss,
    reason = "an integer beyond 2^53 is compared exactly before any float comparison"
)]
const fn widen(number: i128) -> f64 {
    number as f64
}

fn number_matches(ours: &Node, theirs: JsonNumber) -> bool {
    match (ours, theirs) {
        (Node::Int(number), JsonNumber::Signed(other)) => *number == i128::from(other),
        (Node::Int(number), JsonNumber::Unsigned(other)) => *number == i128::from(other),
        (Node::Int(number), JsonNumber::Float(other)) => {
            other.abs() < EXACT_INTEGER && other.fract() == 0.0 && widen(*number) == other
        }
        (Node::Single(number), JsonNumber::Float(other)) => *number == narrow(other),
        (Node::Single(number), JsonNumber::Signed(other)) => *number == narrow(widen(other.into())),
        (Node::Single(number), JsonNumber::Unsigned(other)) => {
            *number == narrow(widen(other.into()))
        }
        (Node::Double(number), JsonNumber::Float(other)) => *number == other,
        (Node::Double(number), JsonNumber::Signed(other)) => *number == widen(other.into()),
        (Node::Double(number), JsonNumber::Unsigned(other)) => *number == widen(other.into()),
        _ => false,
    }
}

/// Whether two Reference values name the same Record: by GUID when the reference side states
/// one, else by the record path against its relative `file://` path.
fn same_target(guid: &str, path: &str, other_guid: &str, other_path: &str) -> bool {
    if !other_guid.is_empty() {
        return guid.eq_ignore_ascii_case(other_guid);
    }
    let mut relative = other_path.strip_prefix("file://").unwrap_or(other_path);
    while let Some(rest) = relative
        .strip_prefix("../")
        .or_else(|| relative.strip_prefix("./"))
    {
        relative = rest;
    }
    relative.eq_ignore_ascii_case(&format!("{path}.json"))
}

fn non_finite_text(text: &str) -> bool {
    matches!(text, "NaN" | "INF" | "-INF")
}

const fn is_non_finite(node: &Node) -> bool {
    match node {
        Node::Single(number) => !number.is_finite(),
        Node::Double(number) => !number.is_finite(),
        _ => false,
    }
}

/// Compares `ours` with the Record a reference tool wrote: type, GUID, name and every field by
/// name, kind and value.
#[must_use]
pub(crate) fn compare_records(ours: &RecordDoc, theirs: &RecordDoc) -> Compared {
    if ours.guid != theirs.guid {
        return Compared::Different(format!("GUID {} against {}", ours.guid, theirs.guid));
    }
    if ours.name != theirs.name {
        return Compared::Different(format!("name {:?} against {:?}", ours.name, theirs.name));
    }
    if ours.tag != theirs.tag {
        return Compared::Different(format!("tag {:?} against {:?}", ours.tag, theirs.tag));
    }
    let mut allowed = None;
    walk(&ours.root, &theirs.root, &mut String::new(), &mut allowed).map_or_else(
        || allowed.map_or(Compared::Equal, Compared::Allowed),
        Compared::Different,
    )
}

fn walk(
    ours: &Node,
    theirs: &Node,
    path: &mut String,
    allowed: &mut Option<&'static str>,
) -> Option<String> {
    let mismatch = |what: &str| Some(format!("{path}: {what}: {ours:?} against {theirs:?}"));
    match (ours, theirs) {
        (Node::Null, Node::Null) | (Node::WeakPointer, Node::WeakPointer) => None,
        (Node::WeakPointer, Node::Null) => allow(allowed, WEAK_NULL),
        (Node::Error(_), Node::Null) => allow(allowed, UNRESOLVED_NULL),
        (Node::Error(_), Node::Bool(_)) => allow(allowed, LENIENT_BOOL),
        (Node::Bool(a), Node::Bool(b)) if a == b => None,
        (Node::Text(a), Node::Text(b)) if a == b => None,
        (Node::Guid(a), Node::Guid(b) | Node::Text(b)) if a.eq_ignore_ascii_case(b) => None,
        (
            Node::Reference { guid, path },
            Node::Reference {
                guid: other_guid,
                path: other_path,
            },
        ) if same_target(guid, path, other_guid, other_path) => None,
        (Node::Single(_) | Node::Double(_), Node::Text(text))
            if is_non_finite(ours) && non_finite_text(text) =>
        {
            None
        }
        (Node::Single(a), Node::Null) if !a.is_finite() => None,
        (Node::Double(a), Node::Null) if !a.is_finite() => None,
        (_, Node::Number(number)) if number_matches(ours, *number) => None,
        (
            Node::Struct {
                type_name: our_type,
                fields: our_fields,
            },
            Node::Struct {
                type_name: their_type,
                fields: their_fields,
            },
        ) => {
            if our_type != their_type {
                return mismatch("struct type");
            }
            walk_fields(our_fields, their_fields, path, allowed)
        }
        (Node::Array(a), Node::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: {} elements against {}", a.len(), b.len()));
            }
            a.iter().zip(b).enumerate().find_map(|(index, (x, y))| {
                let length = path.len();
                let _written = write!(path, "[{index}]");
                let found = walk(x, y, path, allowed);
                path.truncate(length);
                found
            })
        }
        _ => mismatch("value"),
    }
}

fn allow(allowed: &mut Option<&'static str>, reason: &'static str) -> Option<String> {
    *allowed = allowed.or(Some(reason));
    None
}

fn walk_fields(
    ours: &[(String, Node)],
    theirs: &[(String, Node)],
    path: &mut String,
    allowed: &mut Option<&'static str>,
) -> Option<String> {
    if let Some(((name, _), (other, _))) = ours
        .iter()
        .zip(theirs)
        .find(|((name, _), (other, _))| !name.eq_ignore_ascii_case(other))
    {
        return Some(format!("{path}: field {name:?} against {other:?}"));
    }
    if ours.len() != theirs.len() {
        let (side, extra) = if ours.len() > theirs.len() {
            ("P4KX", ours.get(theirs.len()))
        } else {
            ("the reference", theirs.get(ours.len()))
        };
        let name = extra.map_or("", |(name, _)| name.as_str());
        return Some(format!("{path}: field {name:?} only in {side}"));
    }
    ours.iter().zip(theirs).find_map(|((name, x), (_, y))| {
        let length = path.len();
        path.push('.');
        path.push_str(name);
        let found = walk(x, y, path, allowed);
        path.truncate(length);
        found
    })
}
