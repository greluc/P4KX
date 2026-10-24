// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::{DataForgeError, DataForgeLoadError};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use super::{BuiltShipBase, SHIP_BASE_VERSION, ShipBase};

/// The ship base data cannot be built or written.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ShipBaseError {
    /// The DataForge Entry cannot be found or read.
    #[error(transparent)]
    Load(DataForgeLoadError),
    /// The DataForge has an unsupported version or is corrupt.
    #[error(transparent)]
    DataForge(DataForgeError),
    /// The data cannot be written as JSON.
    #[error("cannot write the ship base data as JSON")]
    Json(#[from] serde_json::Error),
    /// The output cannot be written.
    #[error("cannot write the ship base data")]
    Io(#[from] io::Error),
    /// The export was cancelled.
    #[error("the ship base export was cancelled")]
    Cancelled,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Document<'a> {
    schema_version: u32,
    source: &'a Value,
    filter: Option<&'a str>,
    ships: &'a [ShipBase],
}

impl BuiltShipBase {
    /// Writes the data as one JSON document followed by a line feed: the schema version, the
    /// source Archive, the Filter if one narrowed the selection, and the ships.
    ///
    /// # Errors
    ///
    /// Returns [`ShipBaseError::Json`] or [`ShipBaseError::Io`] when writing fails.
    pub fn write_json(&self, out: &mut impl Write) -> Result<(), ShipBaseError> {
        serde_json::to_writer(
            &mut *out,
            &Document {
                schema_version: SHIP_BASE_VERSION,
                source: &self.source,
                filter: self.filter.as_deref(),
                ships: &self.ships,
            },
        )?;
        out.write_all(b"\n")?;
        Ok(())
    }

    /// Writes one ship per line as compact JSON, in the order of [`Self::ships`].
    ///
    /// # Errors
    ///
    /// Returns [`ShipBaseError::Json`] or [`ShipBaseError::Io`] when writing fails.
    pub fn write_ndjson(&self, out: &mut impl Write) -> Result<(), ShipBaseError> {
        for ship in &self.ships {
            serde_json::to_writer(&mut *out, ship)?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }
}
