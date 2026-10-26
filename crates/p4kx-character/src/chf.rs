// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::{Deserialize, Serialize};

use crate::container::{self, Container};
use crate::{Character, ChfError};

/// A CHF character file: the Character and the container that holds it.
///
/// [`Chf::write`] gives back the bytes [`Chf::read`] was given, and so does the JSON form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chf {
    /// The character data.
    pub character: Character,
    /// The header flags and the bytes around the compressed body.
    #[serde(default)]
    pub container: ChfContainer,
}

/// The parts of a CHF file outside the body, kept so an unchanged Character is written back
/// byte-exactly.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChfContainer {
    /// The two header bytes behind the magic.
    #[serde(default)]
    pub flags: u16,
    /// The zstd frame read with the body; written again while it still decompresses to the
    /// body, otherwise the body is compressed anew.
    #[serde(default, with = "crate::json_values::hex")]
    pub frame: Vec<u8>,
    /// The bytes behind the frame up to the end of the file, without trailing zeros; written with
    /// the frame they were read with, and when the body changed, the last 8 bytes of the file.
    #[serde(default, with = "crate::json_values::hex")]
    pub padding: Vec<u8>,
}

impl Chf {
    /// Wraps a Character into a new CHF file with an empty container.
    #[must_use]
    pub fn new(character: Character) -> Self {
        Self {
            character,
            container: ChfContainer::default(),
        }
    }

    /// Reads a CHF file: checks magic, size and CRC-32C, decompresses and parses the body.
    ///
    /// # Errors
    ///
    /// A [`ChfError`] naming what is wrong with the file.
    pub fn read(bytes: &[u8]) -> Result<Self, ChfError> {
        let Container {
            flags,
            frame,
            padding,
            body,
        } = Container::read(bytes)?;
        Ok(Self {
            character: Character::read(&body)?,
            container: ChfContainer {
                flags,
                frame,
                padding,
            },
        })
    }

    /// Writes the CHF file with a correct CRC-32C.
    ///
    /// # Errors
    ///
    /// A [`ChfError`] when the Character cannot be stored or does not fit the file.
    pub fn write(&self) -> Result<Vec<u8>, ChfError> {
        let ChfContainer {
            flags,
            frame,
            padding,
        } = &self.container;
        container::write(*flags, frame, padding, &self.character.write()?)
    }

    /// Returns the JSON form, indented for reading and editing.
    ///
    /// # Errors
    ///
    /// [`ChfError::Json`] when serialisation fails.
    pub fn to_json(&self) -> Result<String, ChfError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parses the JSON form [`Chf::to_json`] writes.
    ///
    /// # Errors
    ///
    /// [`ChfError::Json`] for malformed JSON, unknown fields or out-of-range values. The labels
    /// that [`Chf::to_json_named`] adds are ignored.
    pub fn from_json(json: &[u8]) -> Result<Self, ChfError> {
        Ok(serde_json::from_slice(json)?)
    }
}
