use crate::Language;

const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];
const DECIMAL_UNITS: [&str; 6] = ["kB", "MB", "GB", "TB", "PB", "EB"];

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

/// Formats a byte count as bytes below 1 kB and in decimal units above, with one decimal that is
/// left out when it is zero, e.g. `64 MB` or `57.3 MB`.
pub(crate) fn decimal_size(bytes: u64, language: Language) -> String {
    let mut divisor: u128 = 1;
    let mut unit = None;
    for candidate in DECIMAL_UNITS {
        if u128::from(bytes) < divisor * 1000 {
            break;
        }
        divisor *= 1000;
        unit = Some(candidate);
    }
    let Some(unit) = unit else {
        return format!("{} B", number(bytes, language));
    };
    let tenths = (u128::from(bytes) * 10 + divisor / 2) / divisor;
    let whole = number(u64::try_from(tenths / 10).unwrap_or(u64::MAX), language);
    match tenths % 10 {
        0 => format!("{whole} {unit}"),
        tenth => format!("{whole}{}{tenth} {unit}", decimal_separator(language)),
    }
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
    use super::{decimal_size, number, size};
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

    #[test]
    fn decimal_sizes_leave_out_a_zero_decimal() {
        assert_eq!(decimal_size(999, BritishEnglish), "999 B", "bytes");
        assert_eq!(
            decimal_size(64_000_000, BritishEnglish),
            "64 MB",
            "the upload limit"
        );
        assert_eq!(
            decimal_size(57_250_000, German),
            "57,3 MB",
            "German decimal comma"
        );
        assert_eq!(decimal_size(u64::MAX, BritishEnglish), "18.4 EB", "largest");
    }
}
