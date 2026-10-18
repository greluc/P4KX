use std::fmt;

/// The name of the file an NDJSON Record export writes into its target folder.
pub const NDJSON_FILE: &str = "records.ndjson";

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
}

impl RecordForm {
    /// Every form, in the order they are offered.
    pub const ALL: [Self; 3] = [Self::Xml, Self::Json, Self::Ndjson];

    /// Returns the name of the form: `xml`, `json` or `ndjson`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Xml => "xml",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
        }
    }
}

impl fmt::Display for RecordForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
