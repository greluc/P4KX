use std::io::{self, Read, Write};

use p4kx_cryxml::{Format, Signature, detect, parse};

use crate::{Archive, Entry, LineDiffError};

/// The largest Entry, and the largest XML converted from `CryXml`, a line diff reads.
pub const MAX_DIFF_LEN: u64 = 128 * 1024 * 1024;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";
const UTF16_LE_BOM: &[u8] = b"\xFF\xFE";
const UTF16_BE_BOM: &[u8] = b"\xFE\xFF";

/// How one side of a line diff was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// Text, diffed as it is.
    Text,
    /// `CryXml` with this signature, converted to XML before diffing.
    CryXml(Signature),
}

/// One side of a line diff after reading it.
#[derive(Debug)]
pub(crate) enum Decoded {
    Text(String, Encoding),
    Binary,
    TooLarge,
}

/// Reads `entry` of `archive` as text, converting `CryXml` to XML.
pub(crate) fn decode(archive: &Archive, entry: &Entry) -> Result<Decoded, LineDiffError> {
    if entry.uncompressed_size() > MAX_DIFF_LEN {
        return Ok(Decoded::TooLarge);
    }
    let read_error = |source| LineDiffError::Read {
        path: entry.path().to_owned(),
        source,
    };
    let capacity = usize::try_from(entry.uncompressed_size()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    archive
        .open_entry(entry)?
        .take(MAX_DIFF_LEN)
        .read_to_end(&mut bytes)
        .map_err(read_error)?;
    if let Some(Format::CryXml(signature)) = detect(&bytes) {
        let document = parse(&bytes).map_err(|source| LineDiffError::CryXml {
            path: entry.path().to_owned(),
            source,
        })?;
        let mut out = Capped(Vec::with_capacity(bytes.len().saturating_mul(2)));
        return match document.write_xml(&mut out) {
            Ok(()) => Ok(Decoded::Text(
                into_string(out.0),
                Encoding::CryXml(signature),
            )),
            Err(error) if error.kind() == io::ErrorKind::FileTooLarge => Ok(Decoded::TooLarge),
            Err(error) => Err(read_error(error)),
        };
    }
    Ok(text(bytes).map_or(Decoded::Binary, |text| Decoded::Text(text, Encoding::Text)))
}

/// Returns `bytes` as text: UTF-16 after its byte order mark, otherwise UTF-8 with invalid
/// sequences replaced; `None` when it holds a NUL byte and so is binary.
fn text(bytes: Vec<u8>) -> Option<String> {
    if let Some(units) = bytes.strip_prefix(UTF16_LE_BOM) {
        return Some(utf16(units, u16::from_le_bytes));
    }
    if let Some(units) = bytes.strip_prefix(UTF16_BE_BOM) {
        return Some(utf16(units, u16::from_be_bytes));
    }
    let body = bytes.strip_prefix(UTF8_BOM).unwrap_or(&bytes);
    (!body.contains(&0)).then(|| into_string(bytes))
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let (pairs, rest) = bytes.as_chunks::<2>();
    let mut text: String = char::decode_utf16(pairs.iter().map(|&pair| unit(pair)))
        .map(|decoded| decoded.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    if !rest.is_empty() {
        text.push(char::REPLACEMENT_CHARACTER);
    }
    text
}

fn into_string(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// Collects written bytes up to [`MAX_DIFF_LEN`] and fails with
/// [`io::ErrorKind::FileTooLarge`] beyond it.
struct Capped(Vec<u8>);

impl Write for Capped {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let fits = u64::try_from(self.0.len().saturating_add(buf.len()))
            .is_ok_and(|len| len <= MAX_DIFF_LEN);
        if !fits {
            return Err(io::ErrorKind::FileTooLarge.into());
        }
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
