// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::Ordering;

use crate::{ChangedValue, Field, FieldChange, Location, Step, Value};

/// Returns every value that differs between the fields `old` and `new` of a Record, nested
/// ones included, or that only one side has.
///
/// The changes come depth first: within one struct in field order of `new`, then the fields only
/// `old` has, each followed by the changes inside it. Fields join by name, array elements by
/// index.
pub(crate) fn diff_fields<'a>(old: &[Field<'a>], new: &[Field<'a>]) -> Vec<FieldChange<'a>> {
    let mut walk = Walk::default();
    walk.fields(old, new);
    walk.changes
}

/// The state of one diff: the steps from the Record down to the current value, and the changes
/// so far.
#[derive(Default)]
struct Walk<'a> {
    steps: Vec<Step<'a>>,
    changes: Vec<FieldChange<'a>>,
}

impl<'a> Walk<'a> {
    fn fields(&mut self, old: &[Field<'a>], new: &[Field<'a>]) {
        let same_layout =
            old.len() == new.len() && old.iter().zip(new).all(|(a, b)| a.name() == b.name());
        if same_layout {
            for (a, b) in old.iter().zip(new) {
                self.at(Step::Field(a.name()), Some(a.value()), Some(b.value()));
            }
        } else {
            for (before, after) in join_by_name(old, new) {
                let name = after.or(before).map_or("", Field::name);
                self.at(
                    Step::Field(name),
                    before.map(Field::value),
                    after.map(Field::value),
                );
            }
        }
    }

    fn at(&mut self, step: Step<'a>, old: Option<&Value<'a>>, new: Option<&Value<'a>>) {
        self.steps.push(step);
        self.value(old, new);
        self.steps.pop();
    }

    fn value(&mut self, old: Option<&Value<'a>>, new: Option<&Value<'a>>) {
        match (old, new) {
            (Some(Value::Struct(a)), Some(Value::Struct(b))) => {
                if a.type_name() != b.type_name() {
                    self.push(old, new);
                }
                self.fields(a.fields(), b.fields());
            }
            (Some(Value::Array(a)), Some(Value::Array(b))) => {
                for (index, position) in (0_u32..).zip(0..a.len().max(b.len())) {
                    self.at(Step::Element(index), a.get(position), b.get(position));
                }
            }
            (Some(a), Some(b)) if same(a, b) => {}
            _ => {
                self.push(old, new);
                if let Some(value) = old {
                    self.one_side(value, Lone::Old);
                }
                if let Some(value) = new {
                    self.one_side(value, Lone::New);
                }
            }
        }
    }

    /// Lists the fields or elements of a struct or array only `side` has.
    fn one_side(&mut self, value: &Value<'a>, side: Lone) {
        match value {
            Value::Struct(structure) => {
                for field in structure.fields() {
                    let (old, new) = side.pair(field.value());
                    self.at(Step::Field(field.name()), old, new);
                }
            }
            Value::Array(items) => {
                for (index, item) in (0_u32..).zip(items.iter()) {
                    let (old, new) = side.pair(item);
                    self.at(Step::Element(index), old, new);
                }
            }
            _ => {}
        }
    }

    fn push(&mut self, old: Option<&Value<'a>>, new: Option<&Value<'a>>) {
        self.changes.push(FieldChange {
            path: Location::new(&self.steps),
            old: old.map(ChangedValue::of),
            new: new.map(ChangedValue::of),
        });
    }
}

/// The side that alone has a value.
#[derive(Clone, Copy)]
enum Lone {
    Old,
    New,
}

type Sides<'v, 'a> = (Option<&'v Value<'a>>, Option<&'v Value<'a>>);

impl Lone {
    const fn pair<'v, 'a>(self, value: &'v Value<'a>) -> Sides<'v, 'a> {
        match self {
            Self::Old => (Some(value), None),
            Self::New => (None, Some(value)),
        }
    }
}

type Pair<'f, 'a> = (Option<&'f Field<'a>>, Option<&'f Field<'a>>);

/// Joins the fields by name: those `new` has in its order, then those only `old` has in its.
fn join_by_name<'f, 'a>(old: &'f [Field<'a>], new: &'f [Field<'a>]) -> Vec<Pair<'f, 'a>> {
    let (old_names, new_names) = (by_name(old), by_name(new));
    let mut ordered: Vec<(usize, Pair<'f, 'a>)> = Vec::with_capacity(old.len().max(new.len()));
    let (mut i, mut j) = (0, 0);
    loop {
        let (before, after) = (old_names.get(i), new_names.get(j));
        let order = match (before, after) {
            (None, None) => break,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(before), Some(after)) => before.0.name().cmp(after.0.name()),
        };
        match (order, before, after) {
            (Ordering::Less, Some(&(field, index)), _) => {
                i += 1;
                ordered.push((new.len().saturating_add(index), (Some(field), None)));
            }
            (Ordering::Greater, _, Some(&(field, index))) => {
                j += 1;
                ordered.push((index, (None, Some(field))));
            }
            (_, Some(&(a, _)), Some(&(b, index))) => {
                i += 1;
                j += 1;
                ordered.push((index, (Some(a), Some(b))));
            }
            _ => break,
        }
    }
    ordered.sort_unstable_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, pair)| pair).collect()
}

fn by_name<'f, 'a>(fields: &'f [Field<'a>]) -> Vec<(&'f Field<'a>, usize)> {
    let mut named: Vec<_> = fields.iter().zip(0..).collect();
    named.sort_unstable_by(|a, b| a.0.name().cmp(b.0.name()).then(a.1.cmp(&b.1)));
    named
}

/// Returns whether two values that are not both structs or both arrays are the same.
///
/// Floats compare by their bits, every `NaN` equal to every other, so `-0.0` differs from `0.0`.
/// References compare by target GUID and, for a weak pointer, by the location of the target;
/// values that cannot be read compare by their reason.
fn same(old: &Value<'_>, new: &Value<'_>) -> bool {
    match (old, new) {
        (Value::Single(a), Value::Single(b)) => {
            (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
        }
        (Value::Double(a), Value::Double(b)) => {
            (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
        }
        (Value::Reference(a), Value::Reference(b)) => {
            a.guid() == b.guid() && a.location() == b.location()
        }
        (Value::Struct(_) | Value::Array(_), _) | (_, Value::Struct(_) | Value::Array(_)) => false,
        _ => old == new,
    }
}
