use std::borrow::Cow;

use crate::Language;

/// The localisation keys that stand for no text, matched ignoring ASCII case and the leading `@`.
///
/// A name or description with one of them keeps the key and has no texts.
pub const PLACEHOLDER_KEYS: [&str; 3] = ["@LOC_EMPTY", "@LOC_PLACEHOLDER", "@LOC_UNINITIALIZED"];

/// Returns whether `key` is one of the [`PLACEHOLDER_KEYS`].
#[must_use]
pub fn is_placeholder_key(key: &str) -> bool {
    let key = key.strip_prefix('@').unwrap_or(key);
    PLACEHOLDER_KEYS.iter().any(|placeholder| {
        placeholder
            .strip_prefix('@')
            .is_some_and(|placeholder| placeholder.eq_ignore_ascii_case(key))
    })
}

/// Returns `text` with every `\n` escape the game writes turned into a line feed, borrowed when it
/// has none.
#[must_use]
pub fn decode_line_breaks(text: &str) -> Cow<'_, str> {
    if text.contains("\\n") {
        Cow::Owned(text.replace("\\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// The result of resolving one localisation key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Lookup<'a> {
    /// The key has a non-empty text, in the requested language or in the fallback language.
    Found {
        /// The text as the `global.ini` stores it, with `\n` still escaped; see
        /// [`decode_line_breaks`].
        text: &'a str,
        /// The language the text is in.
        language: Language,
    },
    /// The key is one of the [`PLACEHOLDER_KEYS`] and stands for no text.
    Placeholder,
    /// Neither the requested nor the fallback language has a non-empty text for the key.
    Missing,
}

impl<'a> Lookup<'a> {
    /// Returns the text, or `None` for a placeholder or a missing key.
    #[must_use]
    pub const fn text(&self) -> Option<&'a str> {
        match *self {
            Self::Found { text, .. } => Some(text),
            Self::Placeholder | Self::Missing => None,
        }
    }

    /// Returns the language of the text, or `None` for a placeholder or a missing key.
    #[must_use]
    pub const fn language(&self) -> Option<Language> {
        match *self {
            Self::Found { language, .. } => Some(language),
            Self::Placeholder | Self::Missing => None,
        }
    }

    /// Returns whether the key has no text.
    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
}
