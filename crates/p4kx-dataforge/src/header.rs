// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_foundation::{ByteReader, ParseError};

use crate::record_table::RecordLayout;
use crate::{DataForgeError, ValueKind};

/// The DataForge versions this reader understands.
pub const SUPPORTED_VERSIONS: [u32; 2] = [6, 8];

const PART: &str = "header";

/// The counts and sizes the header gives for every table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Header {
    pub(crate) version: u32,
    pub(crate) words: [u32; 2],
    pub(crate) record_layout: RecordLayout,
    pub(crate) structs: u32,
    pub(crate) properties: u32,
    pub(crate) enums: u32,
    pub(crate) mappings: u32,
    pub(crate) records: u32,
    pub(crate) values: [u32; ValueKind::COUNT],
    pub(crate) text_len: u32,
    pub(crate) blob_len: u32,
}

impl Header {
    /// Reads the header and rejects every version but the [`SUPPORTED_VERSIONS`].
    pub(crate) fn read(reader: &mut ByteReader<'_>) -> Result<Self, DataForgeError> {
        let truncated = |source: ParseError| DataForgeError::Truncated { part: PART, source };
        let mut word = || reader.read_u32().map_err(truncated);
        let _reserved = word()?;
        let version = word()?;
        let Some(record_layout) = RecordLayout::for_version(version) else {
            return Err(DataForgeError::UnsupportedVersion { version });
        };
        let words = [word()?, word()?];
        let [structs, properties, enums, mappings, records] =
            [word()?, word()?, word()?, word()?, word()?];
        let mut values = [0; ValueKind::COUNT];
        for kind in ValueKind::HEADER {
            if let Some(slot) = values.get_mut(kind.index()) {
                *slot = word()?;
            }
        }
        Ok(Self {
            version,
            words,
            record_layout,
            structs,
            properties,
            enums,
            mappings,
            records,
            values,
            text_len: word()?,
            blob_len: word()?,
        })
    }
}
