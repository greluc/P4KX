use std::collections::HashMap;

use crate::LocalizationError;
use crate::key::Key;

/// The largest `global.ini` that is read, 64 MiB; the English one of a Build has about 11 MB.
pub const MAX_GLOBAL_INI_SIZE: u64 = 64 << 20;

/// The most keys a `global.ini` may hold; the English one of a Build has about 91,000.
pub const MAX_GLOBAL_INI_KEYS: usize = 1 << 20;

const BYTE_ORDER_MARK: &[u8] = b"\xef\xbb\xbf";

const BYTES_PER_KEY_ESTIMATE: usize = 128;

/// The texts of one language's `global.ini`, indexed by localisation key.
///
/// Every line `key=value` holds one text; the key ends at the first `=` or `,` (so `ASD_01,P`
/// is the key `ASD_01`) and the value is the rest of the line, kept as the game stores it. Lines
/// without `=` or with an empty key are ignored, and of two lines with the same key the later
/// one wins. Texts are borrowed from the parsed bytes.
#[derive(Debug, Clone)]
pub struct GlobalIni<'a> {
    texts: HashMap<Key<'a>, &'a str>,
}

impl<'a> GlobalIni<'a> {
    /// Indexes the `global.ini` in `bytes`, with or without a UTF-8 byte order mark.
    ///
    /// # Errors
    ///
    /// Returns [`LocalizationError::TooLarge`] when `bytes` exceed [`MAX_GLOBAL_INI_SIZE`],
    /// [`LocalizationError::TooManyKeys`] when they hold more than [`MAX_GLOBAL_INI_KEYS`] keys,
    /// and [`LocalizationError::InvalidUtf8`] when they are not UTF-8.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, LocalizationError> {
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let too_large = LocalizationError::TooLarge {
            size,
            limit: MAX_GLOBAL_INI_SIZE,
        };
        if size > MAX_GLOBAL_INI_SIZE {
            return Err(too_large);
        }
        let body = bytes.strip_prefix(BYTE_ORDER_MARK).unwrap_or(bytes);
        let text = str::from_utf8(body).map_err(|error| LocalizationError::InvalidUtf8 {
            line: line_of(body, error.valid_up_to()),
        })?;
        let mut texts = HashMap::new();
        texts
            .try_reserve((body.len() / BYTES_PER_KEY_ESTIMATE).min(MAX_GLOBAL_INI_KEYS))
            .map_err(|_| too_large)?;
        for line in text.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.split_once(',').map_or(key, |(key, _)| key).trim();
            if key.is_empty() {
                continue;
            }
            if texts.len() == MAX_GLOBAL_INI_KEYS && !texts.contains_key(&Key(key)) {
                return Err(LocalizationError::TooManyKeys {
                    limit: MAX_GLOBAL_INI_KEYS,
                });
            }
            texts.insert(Key(key), value);
        }
        Ok(Self { texts })
    }

    /// Returns the text of the localisation `key`, with or without its leading `@`, ignoring
    /// ASCII case; `None` when the file has no such key.
    #[must_use]
    pub fn resolve(&self, key: &str) -> Option<&'a str> {
        let key = key.strip_prefix('@').unwrap_or(key);
        let texts: &HashMap<Key<'_>, &'a str> = &self.texts;
        texts.get(&Key(key)).copied()
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
}

/// Returns the line, counted from 1, that holds the byte at `offset` of `body`.
fn line_of(body: &[u8], offset: usize) -> usize {
    body.get(..offset)
        .unwrap_or(body)
        .split(|&byte| byte == b'\n')
        .count()
}
