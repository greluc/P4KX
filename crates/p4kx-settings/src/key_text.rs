use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

/// Text meant to be an Archive key, before it is parsed.
///
/// Its `Debug` output hides the text, so that a derived `Debug` of a structure that holds it never
/// prints a key.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct KeyText(String);

impl KeyText {
    /// Wraps `text`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Returns the text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for KeyText {
    type Err = Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(text))
    }
}

impl fmt::Debug for KeyText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyText(<hidden>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_the_text_also_inside_a_derived_debug() {
        #[derive(Debug)]
        struct Holder {
            key: Option<KeyText>,
        }
        let holder = Holder {
            key: Some(KeyText::new("00112233".repeat(4))),
        };

        let shown = format!("{holder:?} {:#?}", holder.key);

        assert!(!shown.contains("00112233"), "{shown}");
        assert!(!shown.contains("eeff"), "{shown}");
    }
}
