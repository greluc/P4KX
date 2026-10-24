// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";
const UTF16_LE_BOM: &[u8] = b"\xFF\xFE";
const UTF16_BE_BOM: &[u8] = b"\xFE\xFF";

/// The signature a `CryXml` file starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signature {
    /// `CryXmlB`, used by current Archives.
    CryXmlB,
    /// `CryXml`.
    CryXml,
    /// `CRY3SDK`.
    Cry3Sdk,
}

impl Signature {
    /// Returns the signature as it appears in the file.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CryXmlB => "CryXmlB",
            Self::CryXml => "CryXml",
            Self::Cry3Sdk => "CRY3SDK",
        }
    }

    pub(crate) fn of(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"CryXmlB\0") {
            Some(Self::CryXmlB)
        } else if bytes.starts_with(b"CryXml\0") && bytes.len() >= Self::CryXml.len() {
            Some(Self::CryXml)
        } else if bytes.starts_with(b"CRY3SDK") && bytes.len() >= Self::Cry3Sdk.len() {
            Some(Self::Cry3Sdk)
        } else {
            None
        }
    }

    pub(crate) const fn len(self) -> usize {
        match self {
            Self::CryXmlB | Self::CryXml => 8,
            Self::Cry3Sdk => 9,
        }
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The byte order of the integers in a `CryXml` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteOrder {
    /// Little-endian.
    Little,
    /// Big-endian.
    Big,
}

impl fmt::Display for ByteOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Little => "little-endian",
            Self::Big => "big-endian",
        })
    }
}

/// What an input holds, as far as its first bytes tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Binary `CryXml` with this signature.
    CryXml(Signature),
    /// Text XML: after an optional byte order mark and whitespace, the first character is `<`.
    Text,
}

/// Detects `CryXml` by its signature, or text XML by its first character.
#[must_use]
pub fn detect(bytes: &[u8]) -> Option<Format> {
    Signature::of(bytes)
        .map(Format::CryXml)
        .or_else(|| is_text(bytes).then_some(Format::Text))
}

fn is_text(bytes: &[u8]) -> bool {
    if let Some(rest) = bytes.strip_prefix(UTF16_LE_BOM) {
        return rest.starts_with(b"<\0");
    }
    if let Some(rest) = bytes.strip_prefix(UTF16_BE_BOM) {
        return rest.starts_with(b"\0<");
    }
    let rest = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    rest.trim_ascii_start().first() == Some(&b'<')
}
