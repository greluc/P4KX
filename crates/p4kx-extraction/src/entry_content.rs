// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Read, Write};

use p4kx_archive::{Archive, ArchiveError, Entry};
use p4kx_cryxml::CryXmlError;
use rayon::prelude::*;
use thiserror::Error;

use crate::convert::is_cryxml;
use crate::entry_path::same_entry_path;
use crate::text_window::{Window, convert_into_window};

/// The fewest bytes of text a read returns while text remains, enough for any one character.
pub const MIN_TEXT_LEN: usize = 4;
const MAX_WHOLE_TEXT_LEN: u64 = 32 << 20;
const MAX_CONVERTED_XML_LEN: u64 = 256 << 20;
const SNIFF_LEN: u64 = 8 << 10;
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const MAX_CONTINUATION_LEN: usize = 3;
const MAX_RESERVED_LEN: u64 = 1 << 20;

/// A failure to read the content of an Entry.
#[derive(Debug, Error)]
pub enum EntryReadError {
    /// The Entry cannot be opened.
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    /// The Entry data cannot be read or decoded.
    #[error("cannot read the Entry data")]
    Read(#[source] io::Error),
    /// The offset lies behind the end of the content.
    #[error("the offset {offset} lies beyond the end at {size}")]
    OffsetBeyondEnd {
        /// The requested offset.
        offset: u64,
        /// The size of the content.
        size: u64,
    },
    /// The Entry holds binary data, not text.
    #[error("the Entry holds binary data, not text")]
    Binary,
    /// The Entry, or the text it decodes to, is too large to decode as a whole.
    #[error("the Entry is too large to decode as a whole: more than {limit} bytes")]
    TooLarge {
        /// The limit in bytes that was exceeded.
        limit: u64,
    },
    /// The `CryXml` is corrupt.
    #[error("cannot convert the CryXml")]
    Convert(#[source] CryXmlError),
}

/// How the text of an Entry is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEncoding {
    /// UTF-8, with or without a byte order mark.
    Utf8,
    /// UTF-16 little endian after its byte order mark.
    Utf16Le,
    /// UTF-16 big endian after its byte order mark.
    Utf16Be,
    /// `CryXml`, converted to UTF-8 XML.
    CryXml,
}

/// A range of the decompressed and decrypted data of an Entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryBytes {
    /// The offset of the first byte in the Entry.
    pub offset: u64,
    /// The bytes read.
    pub bytes: Vec<u8>,
    /// The size of the Entry.
    pub size: u64,
}

/// A range of the text of an Entry, measured in bytes of its UTF-8 form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryText {
    /// How the Entry was decoded.
    pub encoding: TextEncoding,
    /// The offset of the first byte of `text` in the whole text.
    pub offset: u64,
    /// The text read.
    pub text: String,
    /// The size of the whole text in UTF-8 bytes, without a byte order mark.
    pub size: u64,
}

impl EntryBytes {
    /// Returns the offset of the next range, or `None` when the range reaches the end.
    #[must_use]
    pub fn next_offset(&self) -> Option<u64> {
        let end = self.offset.saturating_add(self.bytes.len() as u64);
        (end < self.size).then_some(end)
    }
}

impl EntryText {
    /// Returns the offset of the next range, or `None` when the range reaches the end.
    #[must_use]
    pub fn next_offset(&self) -> Option<u64> {
        let end = self.offset.saturating_add(self.text.len() as u64);
        (end < self.size).then_some(end)
    }
}

/// An encoding that is decoded from the whole Entry.
#[derive(Debug, Clone, Copy)]
enum Whole {
    Utf16Le,
    Utf16Be,
    CryXml,
}

impl Whole {
    fn of(head: &[u8]) -> Option<Self> {
        match head {
            [0xFF, 0xFE, ..] => Some(Self::Utf16Le),
            [0xFE, 0xFF, ..] => Some(Self::Utf16Be),
            start if is_cryxml(start) => Some(Self::CryXml),
            _ => None,
        }
    }

    const fn encoding(self) -> TextEncoding {
        match self {
            Self::Utf16Le => TextEncoding::Utf16Le,
            Self::Utf16Be => TextEncoding::Utf16Be,
            Self::CryXml => TextEncoding::CryXml,
        }
    }
}

/// Returns the Entry of `archive` named `path`, ignoring ASCII case and the separator kind.
#[must_use]
pub fn find_entry<'a>(archive: &'a Archive, path: &str) -> Option<&'a Entry> {
    archive
        .entries()
        .par_iter()
        .find_first(|entry| same_entry_path(entry.path(), path))
}

/// Reads at most `max_len` bytes of the data of `entry` from `offset`.
///
/// # Errors
///
/// Returns [`EntryReadError::OffsetBeyondEnd`] when `offset` exceeds the Entry size, and another
/// [`EntryReadError`] when the data cannot be read.
pub fn read_bytes(
    archive: &Archive,
    entry: &Entry,
    offset: u64,
    max_len: usize,
) -> Result<EntryBytes, EntryReadError> {
    let size = entry.uncompressed_size();
    let remaining = size
        .checked_sub(offset)
        .ok_or(EntryReadError::OffsetBeyondEnd { offset, size })?;
    let mut reader = archive.open_entry_at(entry, offset)?;
    let bytes = read_up_to(&mut reader, remaining.min(max_len as u64))?;
    Ok(EntryBytes {
        offset,
        bytes,
        size,
    })
}

/// Reads the text of `entry` from byte `offset` of its UTF-8 form, at most `max_len` bytes.
///
/// A `max_len` below [`MIN_TEXT_LEN`] counts as that. The range starts and ends on whole
/// characters: an `offset` inside a character moves to the next one. `CryXml` is converted to
/// XML and UTF-16 is recognised by its byte order mark; both are decoded as a whole, and `whole`
/// is called before that and its result held until the range is cut, so a caller can bound how
/// many run at once. Anything else has to be UTF-8 and is read only around the range. NUL,
/// control characters other than tab, line feed, form feed and carriage return, DEL and invalid
/// UTF-8 make data binary, so Latin-1 text with bytes above 127 reads as binary.
///
/// # Errors
///
/// Returns [`EntryReadError::Binary`] for data that is no text, [`EntryReadError::TooLarge`] when
/// `CryXml` or UTF-16 is above 32 MiB or decodes to more than 256 MiB,
/// [`EntryReadError::OffsetBeyondEnd`] when `offset` exceeds the text, and another
/// [`EntryReadError`] when the data cannot be read.
pub fn read_text<G>(
    archive: &Archive,
    entry: &Entry,
    offset: u64,
    max_len: usize,
    whole: impl FnOnce() -> G,
) -> Result<EntryText, EntryReadError> {
    let max_len = max_len.max(MIN_TEXT_LEN);
    let size = entry.uncompressed_size();
    let mut reader = archive.open_entry(entry)?;
    let head = read_up_to(&mut reader, size.min(SNIFF_LEN))?;
    let Some(kind) = Whole::of(&head) else {
        return utf8_range(archive, entry, (head, reader), offset, max_len);
    };
    if size > MAX_WHOLE_TEXT_LEN {
        return Err(EntryReadError::TooLarge {
            limit: MAX_WHOLE_TEXT_LEN,
        });
    }
    let permit = whole();
    let mut data = head;
    let rest = size.saturating_sub(data.len() as u64);
    data.reserve(usize::try_from(rest).unwrap_or(0));
    reader
        .take(rest)
        .read_to_end(&mut data)
        .map_err(EntryReadError::Read)?;
    let keep = max_len.saturating_add(MAX_CONTINUATION_LEN);
    let window = match kind {
        Whole::Utf16Le => utf16(&data, u16::from_le_bytes, offset, keep)?,
        Whole::Utf16Be => utf16(&data, u16::from_be_bytes, offset, keep)?,
        Whole::CryXml => convert_into_window(&data, offset, keep, MAX_CONVERTED_XML_LEN)?,
    };
    drop(data);
    drop(permit);
    let text_size = window.written();
    if offset > text_size {
        return Err(EntryReadError::OffsetBeyondEnd {
            offset,
            size: text_size,
        });
    }
    cut(
        kind.encoding(),
        window.into_kept(),
        offset,
        text_size,
        max_len,
    )
}

fn utf8_range(
    archive: &Archive,
    entry: &Entry,
    (head, reader): (Vec<u8>, impl Read),
    offset: u64,
    max_len: usize,
) -> Result<EntryText, EntryReadError> {
    let size = entry.uncompressed_size();
    if !is_text(&head, head.len() as u64 == size) {
        return Err(EntryReadError::Binary);
    }
    let bom = if head.starts_with(UTF8_BOM) {
        UTF8_BOM.len() as u64
    } else {
        0
    };
    let text_size = size - bom;
    let remaining = text_size
        .checked_sub(offset)
        .ok_or(EntryReadError::OffsetBeyondEnd {
            offset,
            size: text_size,
        })?;
    let window_len = remaining.min((max_len as u64).saturating_add(MAX_CONTINUATION_LEN as u64));
    let start = bom + offset;
    let window = match usize::try_from(start)
        .ok()
        .and_then(|start| head.get(start..))
    {
        Some(buffered) => read_up_to(&mut buffered.chain(reader), window_len)?,
        None => read_up_to(&mut archive.open_entry_at(entry, start)?, window_len)?,
    };
    cut(TextEncoding::Utf8, window, offset, text_size, max_len)
}

/// Cuts the text range out of `window`, the bytes from `offset` of a text of `size` bytes.
fn cut(
    encoding: TextEncoding,
    mut window: Vec<u8>,
    offset: u64,
    size: u64,
    max_len: usize,
) -> Result<EntryText, EntryReadError> {
    let lead = window
        .iter()
        .take(MAX_CONTINUATION_LEN)
        .take_while(|&&byte| is_continuation(byte))
        .count();
    window.truncate(lead.saturating_add(max_len));
    window.drain(..lead);
    let start = offset + lead as u64;
    let at_end = start + window.len() as u64 == size;
    match std::str::from_utf8(&window) {
        Ok(_) => {}
        Err(error) if error.error_len().is_none() && !at_end => {
            window.truncate(error.valid_up_to());
        }
        Err(_) => return Err(EntryReadError::Binary),
    }
    if encoding != TextEncoding::CryXml && window.iter().any(|&byte| is_binary(byte)) {
        return Err(EntryReadError::Binary);
    }
    let text = String::from_utf8(window).map_err(|_| EntryReadError::Binary)?;
    Ok(EntryText {
        encoding,
        offset: start,
        text,
        size,
    })
}

/// Decodes UTF-16 after its byte order mark into a window of its UTF-8 form.
fn utf16(
    data: &[u8],
    unit: fn([u8; 2]) -> u16,
    offset: u64,
    keep: usize,
) -> Result<Window, EntryReadError> {
    let (pairs, odd) = data.get(2..).unwrap_or_default().as_chunks::<2>();
    if !odd.is_empty() {
        return Err(EntryReadError::Binary);
    }
    let mut window = Window::new(offset, keep, MAX_CONVERTED_XML_LEN);
    let mut encoded = [0; 4];
    for decoded in char::decode_utf16(pairs.iter().map(|&pair| unit(pair))) {
        let character = decoded.map_err(|_| EntryReadError::Binary)?;
        if u8::try_from(character).is_ok_and(is_binary) {
            return Err(EntryReadError::Binary);
        }
        window
            .write_all(character.encode_utf8(&mut encoded).as_bytes())
            .map_err(|_| EntryReadError::TooLarge {
                limit: MAX_CONVERTED_XML_LEN,
            })?;
    }
    Ok(window)
}

/// Returns whether `head`, the start of an Entry or all of it when `complete`, is UTF-8 text.
fn is_text(head: &[u8], complete: bool) -> bool {
    if head.iter().any(|&byte| is_binary(byte)) {
        return false;
    }
    match std::str::from_utf8(head) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none() && !complete,
    }
}

/// Returns whether `byte` is a control character that text does not hold.
const fn is_binary(byte: u8) -> bool {
    (byte < 0x20 && !matches!(byte, b'\t' | b'\n' | 0x0C | b'\r')) || byte == 0x7F
}

const fn is_continuation(byte: u8) -> bool {
    byte & 0xC0 == 0x80
}

fn read_up_to(reader: &mut impl Read, len: u64) -> Result<Vec<u8>, EntryReadError> {
    let mut bytes = Vec::with_capacity(usize::try_from(len.min(MAX_RESERVED_LEN)).unwrap_or(0));
    reader
        .by_ref()
        .take(len)
        .read_to_end(&mut bytes)
        .map_err(EntryReadError::Read)?;
    Ok(bytes)
}
