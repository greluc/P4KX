// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

/// The name of the file an NDJSON Record export writes into its target folder.
pub const NDJSON_FILE: &str = "records.ndjson";

/// The name of the file a flat CSV Record export writes into its target folder.
pub const FLAT_CSV_FILE: &str = "values.csv";

/// The name of the file a flat NDJSON Record export writes into its target folder.
pub const FLAT_NDJSON_FILE: &str = "values.ndjson";

/// How a Record export writes the selected Records.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RecordForm {
    /// One Record XML file per Record under its record path.
    #[default]
    Xml,
    /// One pretty Record JSON file per Record under its record path.
    Json,
    /// One file [`NDJSON_FILE`] with one compact Record JSON object per line, in record-path
    /// order.
    Ndjson,
    /// One file [`FLAT_CSV_FILE`] with one CSV row per leaf value (UTF-8 with byte order mark,
    /// formulas neutralised), see `docs/record-flat.md`.
    FlatCsv,
    /// Like [`RecordForm::FlatCsv`] without byte order mark and without changing any value.
    FlatCsvRaw,
    /// One file [`FLAT_NDJSON_FILE`] with one compact JSON object per leaf value.
    FlatNdjson,
}

impl RecordForm {
    /// Every form, in the order they are offered.
    pub const ALL: [Self; 6] = [
        Self::Xml,
        Self::Json,
        Self::Ndjson,
        Self::FlatCsv,
        Self::FlatCsvRaw,
        Self::FlatNdjson,
    ];

    /// Returns the name of the form: `xml`, `json`, `ndjson`, `flat-csv`, `flat-csv-raw` or
    /// `flat-ndjson`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Xml => "xml",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
            Self::FlatCsv => "flat-csv",
            Self::FlatCsvRaw => "flat-csv-raw",
            Self::FlatNdjson => "flat-ndjson",
        }
    }

    /// Returns the one file the form writes into the target folder, or `None` for a form with
    /// one file per Record.
    #[must_use]
    pub const fn file(self) -> Option<&'static str> {
        match self {
            Self::Xml | Self::Json => None,
            Self::Ndjson => Some(NDJSON_FILE),
            Self::FlatCsv | Self::FlatCsvRaw => Some(FLAT_CSV_FILE),
            Self::FlatNdjson => Some(FLAT_NDJSON_FILE),
        }
    }

    /// Returns whether the form is a flat value export.
    #[must_use]
    pub const fn is_flat(self) -> bool {
        matches!(self, Self::FlatCsv | Self::FlatCsvRaw | Self::FlatNdjson)
    }
}

impl fmt::Display for RecordForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
