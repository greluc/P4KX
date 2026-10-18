use std::fmt::{self, Write};

use crate::Value;

/// The most values of a list its short text shows.
pub const LIST_SHOWN: usize = 8;

/// Writes the short text of a Query cell: a scalar in its Record XML text, a list as its first
/// [`LIST_SHOWN`] values in brackets and how many more it holds, e.g.
/// `[1, 2, 3, 4, 5, 6, 7, 8, … 92 more]`.
///
/// # Errors
///
/// Returns the [`fmt::Error`] of writing to `out`.
pub fn write_short_text(out: &mut impl Write, value: &Value<'_>) -> fmt::Result {
    let Value::Array(items) = value else {
        return write!(out, "{value}");
    };
    out.write_char('[')?;
    for (index, item) in items.iter().take(LIST_SHOWN).enumerate() {
        if index > 0 {
            out.write_str(", ")?;
        }
        write!(out, "{item}")?;
    }
    if items.len() > LIST_SHOWN {
        write!(out, ", … {} more", items.len().saturating_sub(LIST_SHOWN))?;
    }
    out.write_char(']')
}

#[cfg(test)]
mod tests {
    use super::write_short_text;
    use crate::Value;

    fn short(value: &Value<'_>) -> String {
        let mut text = String::new();
        let _written = write_short_text(&mut text, value);
        text
    }

    #[test]
    fn a_scalar_is_its_record_xml_text() {
        assert_eq!(short(&Value::Single(2.5)), "2.5", "float");
    }

    #[test]
    fn a_short_list_shows_every_value() {
        let list = Value::Array(vec![Value::Int32(1), Value::Null].into_boxed_slice());

        assert_eq!(short(&list), "[1, null]", "list");
    }

    #[test]
    fn a_long_list_shows_eight_values_and_how_many_more() {
        let list = Value::Array((0..10).map(Value::Int32).collect());

        assert_eq!(
            short(&list),
            "[0, 1, 2, 3, 4, 5, 6, 7, … 2 more]",
            "short form"
        );
    }
}
