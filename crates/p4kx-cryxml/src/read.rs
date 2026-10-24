// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::document::{Document, parse};
use crate::error::CryXmlError;
use crate::format::{Format, detect};

/// An input read as XML: text XML as it is, or a parsed `CryXml` [`Document`].
#[derive(Debug, Clone)]
pub enum Xml<'a> {
    /// Text XML, passed through byte-identical.
    Text(&'a [u8]),
    /// Parsed `CryXml`.
    CryXml(Document<'a>),
}

impl Xml<'_> {
    /// Returns what the input held.
    #[must_use]
    pub const fn format(&self) -> Format {
        match self {
            Self::Text(_) => Format::Text,
            Self::CryXml(document) => Format::CryXml(document.signature()),
        }
    }

    /// Writes the input unchanged when it is text XML, or the document as UTF-8 XML.
    ///
    /// # Errors
    ///
    /// Returns the first error of `out`.
    pub fn write_to<W: Write>(&self, mut out: W) -> io::Result<()> {
        match self {
            Self::Text(bytes) => {
                out.write_all(bytes)?;
                out.flush()
            }
            Self::CryXml(document) => document.write_xml(out),
        }
    }
}

/// Reads `bytes` as text XML or as `CryXml`.
///
/// # Errors
///
/// Returns [`CryXmlError::NotCryXml`] when the input is neither, and the other [`CryXmlError`]
/// variants when `CryXml` is truncated or corrupt.
pub fn read(bytes: &[u8]) -> Result<Xml<'_>, CryXmlError> {
    match detect(bytes) {
        Some(Format::Text) => Ok(Xml::Text(bytes)),
        Some(Format::CryXml(_)) => parse(bytes).map(Xml::CryXml),
        None => Err(CryXmlError::NotCryXml),
    }
}
