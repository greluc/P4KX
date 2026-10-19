use std::collections::HashMap;

use crate::LocalizationError;
use crate::key::Key;

/// The largest `global.ini` that is read, 64 MiB; the English one of a Build has about 11 MB.
pub const MAX_GLOBAL_INI_SIZE: u64 = 64 << 20;

/// The most keys a `global.ini` may hold; the English one of a Build has about 91,000.
pub const MAX_GLOBAL_INI_KEYS: usize = 1 << 20;

const BYTES_PER_KEY_ESTIMATE: usize = 128;

/// The texts of one language's `global.ini`, indexed by localisation key.
///
/// Every line `key=value` holds one text; the key ends at the first `=` or `,` (so `ASD_01,P`
/// is the key `ASD_01`) and the value is the rest of the line, kept as the game stores it. Lines
/// without `=` or with an empty key are skipped without ending the read, and of two lines with
/// the same key the later one wins. Texts are borrowed from the parsed text.
#[derive(Debug, Clone)]
pub struct GlobalIni<'a> {
    texts: HashMap<Key<'a>, &'a str>,
    skipped: usize,
}

impl<'a> GlobalIni<'a> {
    /// Indexes the `global.ini` text `text`; a leading byte order mark is ignored.
    ///
    /// Use [`IniText::decode`](crate::IniText::decode) to get the text of bytes in any of the
    /// encodings the game's files come in.
    ///
    /// # Errors
    ///
    /// Returns [`LocalizationError::TooLarge`] when `text` exceeds [`MAX_GLOBAL_INI_SIZE`],
    /// [`LocalizationError::OutOfMemory`] when the index cannot be reserved and
    /// [`LocalizationError::TooManyKeys`] when it holds more than [`MAX_GLOBAL_INI_KEYS`] keys.
    pub fn parse(text: &'a str) -> Result<Self, LocalizationError> {
        let size = u64::try_from(text.len()).unwrap_or(u64::MAX);
        if size > MAX_GLOBAL_INI_SIZE {
            return Err(LocalizationError::TooLarge {
                size,
                limit: MAX_GLOBAL_INI_SIZE,
            });
        }
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut texts = HashMap::new();
        let keys = (text.len() / BYTES_PER_KEY_ESTIMATE).min(MAX_GLOBAL_INI_KEYS);
        texts
            .try_reserve(keys)
            .map_err(|_| LocalizationError::OutOfMemory { keys })?;
        let mut skipped = 0;
        for line in text.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let key = line.split_once('=').and_then(|(key, value)| {
                let key = key.split_once(',').map_or(key, |(key, _)| key).trim();
                (!key.is_empty()).then_some((key, value))
            });
            let Some((key, value)) = key else {
                skipped += usize::from(!line.trim().is_empty());
                continue;
            };
            if texts.len() == MAX_GLOBAL_INI_KEYS && !texts.contains_key(&Key(key)) {
                return Err(LocalizationError::TooManyKeys {
                    limit: MAX_GLOBAL_INI_KEYS,
                });
            }
            texts.insert(Key(key), value);
        }
        Ok(Self { texts, skipped })
    }

    /// Returns the text of the localisation `key`, with or without its leading `@`, ignoring
    /// ASCII case; `None` when the file has no such key.
    #[must_use]
    pub fn resolve(&self, key: &str) -> Option<&'a str> {
        let key = key.strip_prefix('@').unwrap_or(key);
        let texts: &HashMap<Key<'_>, &'a str> = &self.texts;
        texts.get(&Key(key)).copied()
    }

    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.texts.contains_key(&Key(key))
    }

    /// Returns every key with its text, in no particular order; a key is spelled as its first
    /// line spells it.
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.texts.iter().map(|(key, &text)| (key.0, text))
    }

    /// Returns the number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.texts.len()
    }

    /// Returns whether the file holds no key.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// Returns the number of non-blank lines that were skipped for having no `=` or an empty
    /// key.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }
}
