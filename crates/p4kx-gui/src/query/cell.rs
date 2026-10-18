use std::cmp::Ordering;

use p4kx_extraction::Value;
use p4kx_extraction::query::write_short_text;

use crate::SortDirection;
use crate::sort::compare_ignoring_case;

/// How the window shows a cell of a Query result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellLook {
    /// A value in the row colour.
    Value,
    /// A value that cannot be read, or a list holding one, in the danger text colour.
    Error,
    /// A null, in the muted grey.
    Null,
}

/// One cell of a Query result: its short text and what it sorts by.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cell {
    Number(Box<str>, f64),
    Text(Box<str>),
    List {
        text: Box<str>,
        first: Option<Box<Self>>,
        len: usize,
        error: bool,
    },
    Error(Box<str>),
    Null,
    Empty,
}

impl Cell {
    /// Returns the cell of `value`, a list when `list`; `buffer` is reused for the text.
    pub(crate) fn of(value: Option<&Value<'_>>, list: bool, buffer: &mut String) -> Self {
        match value {
            None => Self::Empty,
            Some(value @ Value::Array(items)) if list => {
                buffer.clear();
                let _infallible = write_short_text(buffer, value);
                Self::List {
                    text: Box::from(buffer.as_str()),
                    first: items
                        .first()
                        .map(|item| Box::new(Self::of(Some(item), false, buffer))),
                    len: items.len(),
                    error: items.iter().any(|item| matches!(item, Value::Error(_))),
                }
            }
            Some(value) => Self::scalar(value, buffer),
        }
    }

    fn scalar(value: &Value<'_>, buffer: &mut String) -> Self {
        if matches!(value, Value::Null) {
            return Self::Null;
        }
        buffer.clear();
        let _infallible = write_short_text(buffer, value);
        let text = Box::from(buffer.as_str());
        match number(value) {
            Some(number) => Self::Number(text, number),
            None if matches!(value, Value::Error(_)) => Self::Error(text),
            None => Self::Text(text),
        }
    }

    /// Returns the text shown: `null` for a null, empty for a missing value.
    pub(crate) fn text(&self) -> &str {
        match self {
            Self::Number(text, _)
            | Self::Text(text)
            | Self::Error(text)
            | Self::List { text, .. } => text,
            Self::Null => "null",
            Self::Empty => "",
        }
    }

    /// Returns how the cell is shown.
    pub(crate) const fn look(&self) -> CellLook {
        match self {
            Self::Error(_) | Self::List { error: true, .. } => CellLook::Error,
            Self::Null => CellLook::Null,
            _ => CellLook::Value,
        }
    }

    const fn rank(&self) -> u8 {
        match self {
            Self::Number(..) => 0,
            Self::Text(_) => 1,
            Self::List { .. } => 2,
            Self::Error(_) => 3,
            Self::Null | Self::Empty => 4,
        }
    }

    /// Orders numbers, texts, lists, unreadable values, then nulls and missing values; within a
    /// kind by value in `direction`: numbers by value, texts ignoring case, lists by their first
    /// value and then their length.
    pub(crate) fn compare(&self, other: &Self, direction: SortDirection) -> Ordering {
        let within = match (self, other) {
            (Self::Number(_, left), Self::Number(_, right)) => left.total_cmp(right),
            (Self::Text(left), Self::Text(right)) | (Self::Error(left), Self::Error(right)) => {
                compare_ignoring_case(left, right)
            }
            (
                Self::List {
                    first: left,
                    len: left_len,
                    ..
                },
                Self::List {
                    first: right,
                    len: right_len,
                    ..
                },
            ) => match (left, right) {
                (Some(left), Some(right)) => left.compare(right, SortDirection::Ascending),
                (left, right) => left.is_some().cmp(&right.is_some()),
            }
            .then(left_len.cmp(right_len)),
            _ => return self.rank().cmp(&other.rank()),
        };
        match direction {
            SortDirection::Ascending => within,
            SortDirection::Descending => within.reverse(),
        }
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "sorting needs the order of magnitude, not every digit of a 64-bit integer"
)]
fn number(value: &Value<'_>) -> Option<f64> {
    Some(match *value {
        Value::Int8(value) => value.into(),
        Value::Int16(value) => value.into(),
        Value::Int32(value) => value.into(),
        Value::Int64(value) => value as f64,
        Value::UInt8(value) => value.into(),
        Value::UInt16(value) => value.into(),
        Value::UInt32(value) => value.into(),
        Value::UInt64(value) => value as f64,
        Value::Single(value) => value.into(),
        Value::Double(value) => value,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use p4kx_extraction::Value;

    use super::{Cell, CellLook};
    use crate::SortDirection;

    fn cell(value: &Value<'_>, list: bool) -> Cell {
        Cell::of(Some(value), list, &mut String::new())
    }

    fn list(values: &[i32]) -> Cell {
        cell(
            &Value::Array(values.iter().copied().map(Value::Int32).collect()),
            true,
        )
    }

    #[test]
    fn a_list_shows_its_short_text() {
        let long: Vec<i32> = (0..10).collect();

        assert_eq!(
            list(&long).text(),
            "[0, 1, 2, 3, 4, 5, 6, 7, … 2 more]",
            "short form"
        );
    }

    #[test]
    fn lists_sort_by_their_first_value_then_by_length() {
        let order = |a: &Cell, b: &Cell| a.compare(b, SortDirection::Ascending);

        assert_eq!(
            order(&list(&[2]), &list(&[10, 1])),
            Ordering::Less,
            "by value"
        );
        assert_eq!(
            order(&list(&[1]), &list(&[1, 0])),
            Ordering::Less,
            "then length"
        );
        assert_eq!(
            order(&list(&[]), &list(&[0])),
            Ordering::Less,
            "empty first"
        );
    }

    #[test]
    fn errors_nulls_and_missing_values_stay_last_in_both_directions() {
        let number = cell(&Value::Int32(1), false);
        let null = cell(&Value::Null, false);
        let empty = Cell::Empty;

        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            assert_eq!(
                number.compare(&null, direction),
                Ordering::Less,
                "{direction:?}"
            );
            assert_eq!(
                number.compare(&empty, direction),
                Ordering::Less,
                "{direction:?}"
            );
            assert_eq!(
                null.compare(&empty, direction),
                Ordering::Equal,
                "{direction:?}"
            );
        }
    }

    #[test]
    fn descending_reverses_within_a_kind_only() {
        let small = cell(&Value::Int32(1), false);
        let large = cell(&Value::Int32(2), false);

        assert_eq!(
            small.compare(&large, SortDirection::Descending),
            Ordering::Greater,
            "reversed"
        );
    }

    #[test]
    fn a_null_is_muted_and_a_list_with_an_error_is_an_error() {
        let broken = Value::Array(
            vec![
                Value::Int32(1),
                Value::Error(p4kx_extraction::ValueError::InvalidBool { byte: 7 }),
            ]
            .into_boxed_slice(),
        );

        assert_eq!(cell(&Value::Null, false).look(), CellLook::Null, "null");
        assert_eq!(cell(&Value::Null, false).text(), "null", "null text");
        assert_eq!(cell(&broken, true).look(), CellLook::Error, "list");
    }
}
