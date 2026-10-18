use std::io::{self, Read, Write};

use p4kx_cryxml::{Format, Signature, detect, parse};

use crate::{Archive, Entry, LineDiffError};

/// The largest Entry, and the largest XML converted from `CryXml`, a line diff reads.
pub const MAX_DIFF_LEN: u64 = 128 * 1024 * 1024;

/// The most lines a side of a line diff may have; with about 9 bytes of bookkeeping per line and
/// side this bounds the memory of a diff at well under 100 MiB beyond the texts.
pub const MAX_DIFF_LINES: usize = 4 * 1024 * 1024;

/// The most a buffer reserves before its data has arrived; it grows from there.
const MAX_INITIAL_CAPACITY: u64 = 16 * 1024 * 1024;

const UTF16_LE_BOM: &[u8] = b"\xFF\xFE";
const UTF16_BE_BOM: &[u8] = b"\xFE\xFF";

/// How one side of a line diff was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// Bytes without a NUL, diffed as stored.
    Text,
    /// UTF-16 little-endian after its byte order mark, decoded to UTF-8.
    Utf16Le,
    /// UTF-16 big-endian after its byte order mark, decoded to UTF-8.
    Utf16Be,
    /// `CryXml` with this signature, converted to XML before diffing.
    CryXml(Signature),
}

impl Encoding {
    /// Returns the name the JSON output uses: `text`, `utf-16le`, `utf-16be` or the `CryXml`
    /// signature.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Utf16Le => "utf-16le",
            Self::Utf16Be => "utf-16be",
            Self::CryXml(signature) => signature.as_str(),
        }
    }
}

/// One side of a line diff after reading it.
#[derive(Debug)]
pub(crate) enum Decoded {
    Text(Vec<u8>, Encoding),
    Binary,
    TooLarge,
}

/// Reads `entry` of `archive` as text, converting `CryXml` to XML and decoding UTF-16.
pub(crate) fn decode(archive: &Archive, entry: &Entry) -> Result<Decoded, LineDiffError> {
    let declared = entry.uncompressed_size();
    if declared > MAX_DIFF_LEN {
        return Ok(Decoded::TooLarge);
    }
    let path = || entry.path().to_owned();
    let mut bytes = Vec::with_capacity(initial_capacity(declared));
    archive
        .open_entry(entry)?
        .take(declared.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| LineDiffError::Read {
            path: path(),
            source,
        })?;
    if bytes.len() as u64 != declared {
        return Err(LineDiffError::SizeMismatch {
            path: path(),
            declared,
        });
    }
    let decoded = match detect(&bytes) {
        Some(Format::CryXml(signature)) => convert(&bytes, signature, entry)?,
        _ => text(bytes),
    };
    Ok(match decoded {
        Decoded::Text(text, _) if line_count(&text) > MAX_DIFF_LINES => Decoded::TooLarge,
        decoded => decoded,
    })
}

fn convert(bytes: &[u8], signature: Signature, entry: &Entry) -> Result<Decoded, LineDiffError> {
    let document = parse(bytes).map_err(|source| LineDiffError::CryXml {
        path: entry.path().to_owned(),
        source,
    })?;
    let mut out = Capped(Vec::with_capacity(initial_capacity(
        (bytes.len() as u64).saturating_mul(2),
    )));
    match document.write_xml(&mut out) {
        Ok(()) => Ok(Decoded::Text(out.0, Encoding::CryXml(signature))),
        Err(error) if error.kind() == io::ErrorKind::FileTooLarge => Ok(Decoded::TooLarge),
        Err(source) => Err(LineDiffError::Read {
            path: entry.path().to_owned(),
            source,
        }),
    }
}

/// Returns `bytes` as text: strict UTF-16 after its byte order mark, otherwise the bytes as they
/// are; binary when they hold a NUL, or when UTF-16 holds a NUL, an unpaired surrogate or an odd
/// trailing byte.
fn text(bytes: Vec<u8>) -> Decoded {
    let utf16 = bytes
        .strip_prefix(UTF16_LE_BOM)
        .map(|units| (utf16(units, u16::from_le_bytes), Encoding::Utf16Le))
        .or_else(|| {
            bytes
                .strip_prefix(UTF16_BE_BOM)
                .map(|units| (utf16(units, u16::from_be_bytes), Encoding::Utf16Be))
        });
    match utf16 {
        Some((Some(text), encoding)) => Decoded::Text(text.into_bytes(), encoding),
        Some((None, _)) => Decoded::Binary,
        None if bytes.contains(&0) => Decoded::Binary,
        None => Decoded::Text(bytes, Encoding::Text),
    }
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Option<String> {
    let (pairs, rest) = bytes.as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let text: String = char::decode_utf16(pairs.iter().map(|&pair| unit(pair)))
        .collect::<Result<_, _>>()
        .ok()?;
    (!text.contains('\0')).then_some(text)
}

fn line_count(text: &[u8]) -> usize {
    text.split_inclusive(|&byte| byte == b'\n').count()
}

fn initial_capacity(expected: u64) -> usize {
    usize::try_from(expected.min(MAX_INITIAL_CAPACITY)).unwrap_or(0)
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
