use std::cmp::Ordering;

use crate::{Field, FieldChange, Value};

/// Returns the fields whose scalar value differs between `old` and `new`, or that only one side
/// has, in field order of `new` and then the fields only `old` has.
///
/// Fields are joined by name; when a name occurs more than once, the occurrences are joined in
/// order. Floats compare by their bits, every `NaN` equal to every other, so `-0.0` differs from
/// `0.0`.
pub(crate) fn diff_fields<'a>(old: &[Field<'a>], new: &[Field<'a>]) -> Vec<FieldChange<'a>> {
    let same_layout =
        old.len() == new.len() && old.iter().zip(new).all(|(a, b)| a.name() == b.name());
    if same_layout {
        old.iter()
            .zip(new)
            .filter(|(a, b)| !same(a.value(), b.value()))
            .map(|(a, b)| change(a.name(), Some(a.value()), Some(b.value())))
            .collect()
    } else {
        join_by_name(old, new)
    }
}

fn join_by_name<'a>(old: &[Field<'a>], new: &[Field<'a>]) -> Vec<FieldChange<'a>> {
    let (old_names, new_names) = (by_name(old), by_name(new));
    let mut ordered: Vec<(usize, FieldChange<'a>)> = Vec::new();
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
                if is_scalar(field.value()) {
                    ordered.push((
                        new.len().saturating_add(index),
                        change(field.name(), Some(field.value()), None),
                    ));
                }
            }
            (Ordering::Greater, _, Some(&(field, index))) => {
                j += 1;
                if is_scalar(field.value()) {
                    ordered.push((index, change(field.name(), None, Some(field.value()))));
                }
            }
            (_, Some(&(a, _)), Some(&(b, index))) => {
                i += 1;
                j += 1;
                if !same(a.value(), b.value()) {
                    ordered.push((index, change(b.name(), Some(a.value()), Some(b.value()))));
                }
            }
            _ => break,
        }
    }
    ordered.sort_unstable_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, change)| change).collect()
}

fn by_name<'f, 'a>(fields: &'f [Field<'a>]) -> Vec<(&'f Field<'a>, usize)> {
    let mut named: Vec<_> = fields.iter().zip(0..).collect();
    named.sort_unstable_by(|a, b| a.0.name().cmp(b.0.name()).then(a.1.cmp(&b.1)));
    named
}

fn change<'a>(path: &'a str, old: Option<&Value<'a>>, new: Option<&Value<'a>>) -> FieldChange<'a> {
    FieldChange {
        path,
        old: old.cloned(),
        new: new.cloned(),
    }
}

/// Returns whether two values are the same; values that are not scalar are not compared here.
fn same(old: &Value<'_>, new: &Value<'_>) -> bool {
    match (old, new) {
        _ if !is_scalar(old) || !is_scalar(new) => true,
        (Value::Single(a), Value::Single(b)) => {
            (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
        }
        (Value::Double(a), Value::Double(b)) => {
            (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
        }
        _ => old == new,
    }
}

/// Returns whether `value` is a scalar or an error value, the kinds this level compares.
const fn is_scalar(value: &Value<'_>) -> bool {
    matches!(
        value,
        Value::Bool(_)
            | Value::Int8(_)
            | Value::Int16(_)
            | Value::Int32(_)
            | Value::Int64(_)
            | Value::UInt8(_)
            | Value::UInt16(_)
            | Value::UInt32(_)
            | Value::UInt64(_)
            | Value::Single(_)
            | Value::Double(_)
            | Value::String(_)
            | Value::Locale(_)
            | Value::Guid(_)
            | Value::Enum(_)
            | Value::Error(_)
    )
}
