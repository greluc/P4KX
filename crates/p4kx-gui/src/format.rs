use crate::Language;

const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];

/// Formats `value` with the digit grouping of `language`, e.g. `1,234` or `1.234`.
pub(crate) fn number(value: u64, language: Language) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (position, digit) in digits.chars().enumerate() {
        if position > 0 && (digits.len() - position).is_multiple_of(3) {
            grouped.push(group_separator(language));
        }
        grouped.push(digit);
    }
    grouped
}

/// Formats a byte count as bytes below 1 KiB and with one decimal in binary units above.
pub(crate) fn size(bytes: u64, language: Language) -> String {
    let mut divisor: u128 = 1;
    let mut unit = None;
    for candidate in UNITS {
        if u128::from(bytes) < divisor * 1024 {
            break;
        }
        divisor *= 1024;
        unit = Some(candidate);
    }
    let Some(unit) = unit else {
        return format!("{} B", number(bytes, language));
    };
    let tenths = (u128::from(bytes) * 10 + divisor / 2) / divisor;
    let whole = u64::try_from(tenths / 10).unwrap_or(u64::MAX);
    format!(
        "{}{}{} {unit}",
        number(whole, language),
        decimal_separator(language),
        tenths % 10
    )
}

const fn group_separator(language: Language) -> char {
    match language {
        Language::German => '.',
        Language::BritishEnglish => ',',
    }
}

const fn decimal_separator(language: Language) -> char {
    match language {
        Language::German => ',',
        Language::BritishEnglish => '.',
    }
}

#[cfg(test)]
mod tests {
    use super::{number, size};
    use crate::Language::{BritishEnglish, German};

    #[test]
    fn numbers_are_grouped_by_thousands() {
        assert_eq!(number(0, BritishEnglish), "0", "zero");
        assert_eq!(number(999, BritishEnglish), "999", "no group");
        assert_eq!(number(1_234_567, BritishEnglish), "1,234,567", "en-GB");
        assert_eq!(number(1_234_567, German), "1.234.567", "de-DE");
    }

    #[test]
    fn sizes_use_bytes_then_binary_units_with_one_decimal() {
        assert_eq!(size(512, BritishEnglish), "512 B", "bytes");
        assert_eq!(size(1024, BritishEnglish), "1.0 KiB", "one KiB");
        assert_eq!(size(1536, German), "1,5 KiB", "German decimal comma");
        assert_eq!(
            size(5 * 1024 * 1024 * 1024, BritishEnglish),
            "5.0 GiB",
            "GiB"
        );
        assert_eq!(size(u64::MAX, BritishEnglish), "16,384.0 PiB", "largest");
    }
}
