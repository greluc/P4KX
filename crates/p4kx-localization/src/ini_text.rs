use std::borrow::Cow;

use crate::LocalizationError;
use crate::global_ini::{GlobalIni, MAX_GLOBAL_INI_SIZE};

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
const UTF16_LE_BOM: &[u8] = b"\xff\xfe";
const UTF16_BE_BOM: &[u8] = b"\xfe\xff";

/// The encoding a `global.ini` was stored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IniEncoding {
    /// UTF-8 without a byte order mark.
    Utf8,
    /// UTF-8 with a byte order mark, as the game stores its files.
    Utf8Bom,
    /// UTF-16, little endian.
    Utf16Le,
    /// UTF-16, big endian.
    Utf16Be,
}

/// The text of a `global.ini` decoded to UTF-8, borrowed from the bytes where they already are.
///
/// UTF-8 with or without a byte order mark and UTF-16 of either byte order, with a byte order
/// mark or, when the file starts with an ASCII character, without, are read. Bytes that are no
/// valid text become U+FFFD instead of ending the read, so one malformed line costs only itself.
#[derive(Debug, Clone)]
pub struct IniText<'a> {
    text: Cow<'a, str>,
    encoding: IniEncoding,
    lossy: bool,
}

impl<'a> IniText<'a> {
    /// Decodes `bytes`, borrowing them when they are valid UTF-8.
    ///
    /// # Errors
    ///
    /// Returns [`LocalizationError::TooLarge`] when `bytes` exceed [`MAX_GLOBAL_INI_SIZE`] and
    /// [`LocalizationError::DecodeOutOfMemory`] when the decoded text cannot be reserved.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, LocalizationError> {
        check_size(bytes.len())?;
        let (encoding, body) = detect(bytes);
        if matches!(encoding, IniEncoding::Utf16Le | IniEncoding::Utf16Be) {
            let (text, lossy) = decode_utf16(body, encoding == IniEncoding::Utf16Be)?;
            return Ok(Self {
                text: Cow::Owned(text),
                encoding,
                lossy,
            });
        }
        let text = String::from_utf8_lossy(body);
        let lossy = matches!(text, Cow::Owned(_));
        Ok(Self {
            text,
            encoding,
            lossy,
        })
    }

    /// Decodes the owned `bytes`, reusing their allocation when they are valid UTF-8.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`IniText::decode`].
    pub fn decode_vec(bytes: Vec<u8>) -> Result<IniText<'static>, LocalizationError> {
        check_size(bytes.len())?;
        let (encoding, body) = detect(&bytes);
        let prefix = bytes.len() - body.len();
        if matches!(encoding, IniEncoding::Utf8 | IniEncoding::Utf8Bom) {
            match String::from_utf8(bytes) {
                Ok(mut text) => {
                    text.replace_range(..prefix, "");
                    return Ok(IniText {
                        text: Cow::Owned(text),
                        encoding,
                        lossy: false,
                    });
                }
                Err(error) => return IniText::decode(error.as_bytes()).map(IniText::into_owned),
            }
        }
        IniText::decode(&bytes).map(IniText::into_owned)
    }

    fn into_owned(self) -> IniText<'static> {
        IniText {
            text: Cow::Owned(self.text.into_owned()),
            encoding: self.encoding,
            lossy: self.lossy,
        }
    }

    /// Returns the decoded text, without the byte order mark.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the encoding the bytes were stored in.
    #[must_use]
    pub const fn encoding(&self) -> IniEncoding {
        self.encoding
    }

    /// Returns whether bytes that are no valid text were replaced by U+FFFD.
    #[must_use]
    pub const fn is_lossy(&self) -> bool {
        self.lossy
    }

    /// Indexes the texts by localisation key.
    ///
    /// # Errors
    ///
    /// Returns a [`LocalizationError`] when the file holds more keys than the limit or the index
    /// cannot be reserved.
    pub fn index(&self) -> Result<GlobalIni<'_>, LocalizationError> {
        GlobalIni::parse(&self.text)
    }
}

fn check_size(len: usize) -> Result<(), LocalizationError> {
    let size = u64::try_from(len).unwrap_or(u64::MAX);
    if size > MAX_GLOBAL_INI_SIZE {
        return Err(LocalizationError::TooLarge {
            size,
            limit: MAX_GLOBAL_INI_SIZE,
        });
    }
    Ok(())
}

fn detect(bytes: &[u8]) -> (IniEncoding, &[u8]) {
    let boms = [
        (UTF8_BOM, IniEncoding::Utf8Bom),
        (UTF16_LE_BOM, IniEncoding::Utf16Le),
        (UTF16_BE_BOM, IniEncoding::Utf16Be),
    ];
    for (bom, encoding) in boms {
        if let Some(body) = bytes.strip_prefix(bom) {
            return (encoding, body);
        }
    }
    match bytes {
        [first, 0, ..] if *first != 0 => (IniEncoding::Utf16Le, bytes),
        [0, second, ..] if *second != 0 => (IniEncoding::Utf16Be, bytes),
        _ => (IniEncoding::Utf8, bytes),
    }
}
fn decode_utf16(body: &[u8], big_endian: bool) -> Result<(String, bool), LocalizationError> {
    let (pairs, rest) = body.as_chunks::<2>();
    let mut text = String::new();
    text.try_reserve(pairs.len())
        .map_err(|_| LocalizationError::DecodeOutOfMemory {
            size: u64::try_from(body.len()).unwrap_or(u64::MAX),
        })?;
    let units = pairs.iter().map(|&pair| {
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    });
    let mut lossy = !rest.is_empty();
    for unit in char::decode_utf16(units) {
        text.push(unit.unwrap_or_else(|_| {
            lossy = true;
            char::REPLACEMENT_CHARACTER
        }));
    }
    Ok((text, lossy))
}
