use std::io::{self, Read};

use p4kx_archive::{Archive, ArchiveError, Entry};
use p4kx_cryxml::{self as cryxml, CryXmlError, Format};
use thiserror::Error;

use crate::entry_path::same_entry_path;

/// The largest Entry whose text is decoded as a whole: `CryXml` converted to XML and UTF-16 text.
pub const MAX_WHOLE_TEXT_LEN: u64 = 64 << 20;
/// The fewest bytes of text a read returns while text remains, enough for any one character.
pub const MIN_TEXT_LEN: usize = 4;
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
    /// The Entry is too large to decode as a whole.
    #[error("the Entry is {size} bytes, more than the {limit} bytes decoded as a whole")]
    TooLarge {
        /// The Entry size.
        size: u64,
        /// [`MAX_WHOLE_TEXT_LEN`].
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

/// Returns the Entry of `archive` named `path`, ignoring ASCII case and the separator kind.
#[must_use]
pub fn find_entry<'a>(archive: &'a Archive, path: &str) -> Option<&'a Entry> {
    archive
        .entries()
        .iter()
        .find(|entry| same_entry_path(entry.path(), path))
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
    let mut reader = archive.open_entry(entry)?;
    skip(&mut reader, offset)?;
    let len = remaining.min(max_len as u64);
    let bytes = read_up_to(&mut reader, len)?;
    Ok(EntryBytes {
        offset,
        bytes,
        size,
    })
}

/// Reads the text of `entry` from byte `offset` of its UTF-8 form, at most `max_len` bytes.
///
/// A `max_len` below [`MIN_TEXT_LEN`] counts as that. The range starts and ends on whole
/// characters: an `offset` inside a character moves to the next one.
/// `CryXml` is converted to XML, UTF-16 is recognised by its byte order mark, anything else has to
/// be UTF-8 without NUL characters.
///
/// # Errors
///
/// Returns [`EntryReadError::Binary`] for data that is no text, [`EntryReadError::TooLarge`] for
/// `CryXml` or UTF-16 above [`MAX_WHOLE_TEXT_LEN`], [`EntryReadError::OffsetBeyondEnd`] when
/// `offset` exceeds the text, and another [`EntryReadError`] when the data cannot be read.
pub fn read_text(
    archive: &Archive,
    entry: &Entry,
    offset: u64,
    max_len: usize,
) -> Result<EntryText, EntryReadError> {
    let max_len = max_len.max(MIN_TEXT_LEN);
    let size = entry.uncompressed_size();
    let mut reader = archive.open_entry(entry)?;
    let head = read_up_to(&mut reader, size.min(SNIFF_LEN))?;
    let whole = match head.as_slice() {
        [0xFF, 0xFE, ..] => Some(TextEncoding::Utf16Le),
        [0xFE, 0xFF, ..] => Some(TextEncoding::Utf16Be),
        start if matches!(cryxml::detect(start), Some(Format::CryXml(_))) => {
            Some(TextEncoding::CryXml)
        }
        _ => None,
    };
    let Some(encoding) = whole else {
        return utf8_range(&head, reader, size, offset, max_len);
    };
    if size > MAX_WHOLE_TEXT_LEN {
        return Err(EntryReadError::TooLarge {
            size,
            limit: MAX_WHOLE_TEXT_LEN,
        });
    }
    let mut data = head;
    let rest = size.saturating_sub(data.len() as u64);
    data.reserve(usize::try_from(rest).unwrap_or(0));
    reader
        .take(rest)
        .read_to_end(&mut data)
        .map_err(EntryReadError::Read)?;
    let text = match encoding {
        TextEncoding::Utf16Le => utf16(data.get(2..).unwrap_or_default(), u16::from_le_bytes)?,
        TextEncoding::Utf16Be => utf16(data.get(2..).unwrap_or_default(), u16::from_be_bytes)?,
        TextEncoding::CryXml | TextEncoding::Utf8 => converted(&data)?,
    };
    slice_text(encoding, text, offset, max_len)
}

fn utf8_range(
    head: &[u8],
    reader: impl Read,
    size: u64,
    offset: u64,
    max_len: usize,
) -> Result<EntryText, EntryReadError> {
    let complete = head.len() as u64 == size;
    if !is_text(head, complete) {
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
    let mut data = head.chain(reader);
    skip(&mut data, bom + offset)?;
    let window_len = remaining.min((max_len as u64).saturating_add(MAX_CONTINUATION_LEN as u64));
    let mut window = read_up_to(&mut data, window_len)?;
    let lead = window
        .iter()
        .take(MAX_CONTINUATION_LEN)
        .take_while(|&&byte| is_continuation(byte))
        .count();
    window.truncate(lead.saturating_add(max_len));
    window.drain(..lead);
    let at_end = (lead + window.len()) as u64 == remaining;
    match std::str::from_utf8(&window) {
        Ok(_) => {}
        Err(error) if error.error_len().is_none() && !at_end => {
            window.truncate(error.valid_up_to());
        }
        Err(_) => return Err(EntryReadError::Binary),
    }
    if window.contains(&0) {
        return Err(EntryReadError::Binary);
    }
    let text = String::from_utf8(window).map_err(|_| EntryReadError::Binary)?;
    Ok(EntryText {
        encoding: TextEncoding::Utf8,
        offset: offset + lead as u64,
        text,
        size: text_size,
    })
}

fn slice_text(
    encoding: TextEncoding,
    mut text: String,
    offset: u64,
    max_len: usize,
) -> Result<EntryText, EntryReadError> {
    let size = text.len() as u64;
    let start = usize::try_from(offset)
        .ok()
        .filter(|&start| start <= text.len())
        .ok_or(EntryReadError::OffsetBeyondEnd { offset, size })?;
    let start = text.ceil_char_boundary(start);
    let end = text.floor_char_boundary(start.saturating_add(max_len));
    text.truncate(end);
    text.drain(..start);
    Ok(EntryText {
        encoding,
        offset: start as u64,
        text,
        size,
    })
}

fn converted(data: &[u8]) -> Result<String, EntryReadError> {
    let document = cryxml::parse(data).map_err(EntryReadError::Convert)?;
    let mut xml = Vec::with_capacity(data.len());
    document.write_xml(&mut xml).map_err(EntryReadError::Read)?;
    String::from_utf8(xml).map_err(|_| EntryReadError::Binary)
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Result<String, EntryReadError> {
    let (pairs, odd) = bytes.as_chunks::<2>();
    if !odd.is_empty() {
        return Err(EntryReadError::Binary);
    }
    let text = char::decode_utf16(pairs.iter().map(|&pair| unit(pair)))
        .collect::<Result<String, _>>()
        .map_err(|_| EntryReadError::Binary)?;
    if text.contains('\0') {
        return Err(EntryReadError::Binary);
    }
    Ok(text)
}

/// Returns whether `head`, the start of an Entry or all of it when `complete`, is UTF-8 without
/// NUL characters.
fn is_text(head: &[u8], complete: bool) -> bool {
    if head.contains(&0) {
        return false;
    }
    match std::str::from_utf8(head) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none() && !complete,
    }
}

const fn is_continuation(byte: u8) -> bool {
    byte & 0xC0 == 0x80
}

fn skip(reader: &mut impl Read, len: u64) -> Result<(), EntryReadError> {
    let skipped =
        io::copy(&mut reader.by_ref().take(len), &mut io::sink()).map_err(EntryReadError::Read)?;
    if skipped == len {
        Ok(())
    } else {
        Err(EntryReadError::Read(io::ErrorKind::UnexpectedEof.into()))
    }
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
