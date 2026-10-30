// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ffi::OsStr;

use clap::builder::{PossibleValue, TypedValueParser};
use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, Args, Command};
use p4kx_extraction::HircKind;

use crate::cli_bnk::BnkSource;

/// Arguments of `p4kx bnk objects`.
#[derive(Debug, Args)]
pub(crate) struct BnkObjectsArgs {
    /// The soundbank.
    #[command(flatten)]
    pub(crate) source: BnkSource,
    /// Lists only objects of this type.
    #[arg(long = "type", value_name = "TYPE", value_parser = KindParser)]
    pub(crate) kind: Option<HircKind>,
    /// Lists only the object with this ID, in decimal or as 0x followed by hexadecimal.
    #[arg(long, value_name = "ID", value_parser = parse_id)]
    pub(crate) id: Option<u32>,
    /// The most objects listed.
    #[arg(
        long,
        default_value_t = 100,
        value_name = "COUNT",
        value_parser = clap::value_parser!(u32).range(1..=10_000)
    )]
    pub(crate) limit: u32,
    /// The number of objects to skip, as the summary line of the previous page gives it.
    #[arg(long, default_value_t = 0, value_name = "COUNT")]
    pub(crate) offset: u32,
}

/// Parses the values of `--type`: the slugs of [`HircKind::known`].
#[derive(Clone, Copy)]
struct KindParser;

impl TypedValueParser for KindParser {
    type Value = HircKind;

    fn parse_ref(
        &self,
        command: &Command,
        argument: Option<&Arg>,
        value: &OsStr,
    ) -> Result<HircKind, clap::Error> {
        value.to_str().and_then(HircKind::from_slug).ok_or_else(|| {
            let mut error = clap::Error::new(ErrorKind::InvalidValue).with_cmd(command);
            if let Some(argument) = argument {
                error.insert(
                    ContextKind::InvalidArg,
                    ContextValue::String(argument.to_string()),
                );
            }
            error.insert(
                ContextKind::InvalidValue,
                ContextValue::String(value.to_string_lossy().into_owned()),
            );
            error.insert(
                ContextKind::ValidValue,
                ContextValue::Strings(
                    HircKind::known()
                        .map(|kind| kind.slug().to_owned())
                        .collect(),
                ),
            );
            error
        })
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            HircKind::known().map(|kind| PossibleValue::new(kind.slug())),
        ))
    }
}

const HEX_PREFIX: &str = "0x";

/// Parses decimal digits or `0x` and hexadecimal digits, without a sign.
fn parse_id(text: &str) -> Result<u32, String> {
    let (digits, radix) = text
        .strip_prefix(HEX_PREFIX)
        .map_or((text, 10), |hex| (hex, 16));
    if digits.is_empty() || !digits.chars().all(|digit| digit.is_digit(radix)) {
        return Err(format!("{text:?} is not an object ID"));
    }
    u32::from_str_radix(digits, radix)
        .map_err(|error| format!("{text:?} is not an object ID: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_type_is_the_slug_of_a_known_kind() {
        let parser = KindParser;
        let command = Command::new("test");
        for kind in HircKind::known() {
            let parsed = parser.parse_ref(&command, None, OsStr::new(kind.slug()));
            assert_eq!(parsed.ok(), Some(kind), "{kind}");
        }
        for text in ["other", "Sound", ""] {
            let parsed = parser.parse_ref(&command, None, OsStr::new(text));
            assert!(parsed.is_err(), "{text:?} is no type");
        }
    }

    #[test]
    fn ids_are_decimal_or_hexadecimal() {
        assert_eq!(parse_id("255"), Ok(255));
        assert_eq!(parse_id("0xFF"), Ok(255));
        assert!(parse_id("0x").is_err());
        assert!(parse_id("-1").is_err());
        assert!(parse_id("+5").is_err());
        assert!(parse_id("0x+ff").is_err());
        assert!(parse_id("0x-1").is_err());
        assert!(parse_id("").is_err());
        assert!(parse_id("4294967296").is_err());
    }
}
