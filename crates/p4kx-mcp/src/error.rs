use std::error::Error;
use std::path::PathBuf;

use p4kx_extraction::{ArchiveError, DataForgeError, EntryReadError};
use p4kx_settings::SettingsError;
use thiserror::Error;

/// A tool call that cannot be answered; its message goes back to the client as a tool error.
#[derive(Debug, Error)]
pub(crate) enum ToolError {
    /// Neither the call nor the server argument names an Archive.
    #[error("no Archive given: pass `archive` or start the server with `--archive`")]
    NoArchive,
    /// The text names no detected Channel.
    #[error("`{name}` is no detected Channel; detected: {detected}")]
    UnknownChannel {
        /// The Channel name asked for.
        name: String,
        /// The detected Channel names, comma separated.
        detected: String,
    },
    /// The text is no absolute local path of a `.p4k` file.
    #[error("`{0}` is neither a Channel name nor the absolute local path of a `.p4k` file")]
    NotAnArchivePath(String),
    /// The path names no existing file.
    #[error("{} does not exist or is not a file", .0.display())]
    Missing(PathBuf),
    /// The path resolves to a network drive, a device or a file that is no `.p4k`.
    #[error("{} does not resolve to a `.p4k` file on a local drive", .0.display())]
    NotLocal(PathBuf),
    /// An argument exceeds its size cap.
    #[error("`{name}` is longer than {max} bytes")]
    TooLong {
        /// The argument name.
        name: &'static str,
        /// The cap in bytes.
        max: usize,
    },
    /// The cursor is not one this tool returned for this listing.
    #[error("the cursor `{0}` is not one this listing returned")]
    BadCursor(String),
    /// The Archive cannot be opened.
    #[error("cannot open the Archive {}", .path.display())]
    Open {
        /// The Archive path.
        path: PathBuf,
        /// What failed.
        #[source]
        source: ArchiveError,
    },
    /// The Archive holds no Entry with the path.
    #[error("the Archive holds no Entry `{0}`; search_entries finds Entries by path")]
    UnknownEntry(String),
    /// The Entry holds binary data where text was asked for.
    #[error("the Entry {0} holds binary data, not text; read it with read_entry_bytes")]
    Binary(String),
    /// The Entry is too large to decode as text.
    #[error(
        "the Entry {path} is too large to read as text: more than {limit} bytes to decode; read it with read_entry_bytes"
    )]
    TooLarge {
        /// The Entry path.
        path: String,
        /// The limit in bytes.
        limit: u64,
    },
    /// The Entry content cannot be read.
    #[error("cannot read the Entry {path}")]
    Read {
        /// The Entry path.
        path: String,
        /// What failed.
        #[source]
        source: EntryReadError,
    },
    /// The settings file with the manual Channel entries cannot be read.
    #[error("cannot read the settings")]
    Settings(#[from] SettingsError),
    /// The worker running the call ended without a result.
    #[error("the tool call stopped unexpectedly")]
    Stopped,
    /// The DataForge of the Archive cannot be read; the message holds every cause.
    #[error("cannot read the DataForge: {0}")]
    NoDataForge(String),
    /// A value of the DataForge cannot be read.
    #[error("the DataForge cannot be read")]
    DataForge(#[from] DataForgeError),
    /// The call names neither a GUID nor a record path, or both.
    #[error("pass either `guid` or `path`")]
    RecordSelector,
    /// The text is no GUID.
    #[error("`{0}` is no GUID; write it as 01234567-89ab-cdef-0123-456789abcdef")]
    BadGuid(String),
    /// No Record has the GUID.
    #[error("no Record has the GUID {0}; search_records finds Records")]
    UnknownGuid(String),
    /// No Record has the record path.
    #[error("no Record has the record path `{0}`; search_records finds Records")]
    UnknownRecordPath(String),
    /// One item alone is larger than a response may be.
    #[error("the {what} is larger than the response cap of {limit} bytes; {hint}")]
    TooLargeItem {
        /// What is too large, e.g. `Record EntityClassDefinition.AEGS_Avenger`.
        what: String,
        /// The cap in bytes.
        limit: usize,
        /// How to ask for less.
        hint: &'static str,
    },
    /// The Query selects more columns or compares more values than a call may.
    #[error("the Query has {count} {what}, more than the {max} a call may have; split it")]
    QueryTooComplex {
        /// What there is too much of.
        what: &'static str,
        /// How many there are.
        count: usize,
        /// The most there may be.
        max: usize,
    },
    /// Selecting the Records of the Query took too long or the call was cancelled.
    #[error(
        "selecting the Records was stopped after at most {0} seconds or by the client; narrow the Query with `from`, `at` or `where`"
    )]
    QueryStopped(u64),
    /// A result cannot be converted to JSON.
    #[error("cannot convert the result to JSON")]
    Json(#[from] serde_json::Error),
    /// A result is not valid UTF-8.
    #[error("the result is not valid UTF-8")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// A result cannot be written.
    #[error("cannot write the result")]
    Write(#[from] std::io::Error),
    /// The Query cannot be parsed or compiled; the message marks the span.
    #[error("the Query is invalid at bytes {start}..{end}: {marked}")]
    Query {
        /// The offset of the first byte of the span.
        start: usize,
        /// The offset behind the span.
        end: usize,
        /// The message with the marked line of the Query.
        marked: String,
    },
}

impl ToolError {
    /// Returns the message with every cause, `: ` separated.
    pub(crate) fn message(&self) -> String {
        let mut message = chain(self);
        if let Some(hint) = crate::keys::hint(self) {
            message.push_str(". ");
            message.push_str(hint);
        }
        message
    }
}

/// Returns the message of `error` with every cause, `: ` separated.
pub(crate) fn chain(error: &dyn Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// Returns `text` when it fits `max` bytes.
pub(crate) const fn capped<'a>(
    name: &'static str,
    text: &'a str,
    max: usize,
) -> Result<&'a str, ToolError> {
    if text.len() > max {
        Err(ToolError::TooLong { name, max })
    } else {
        Ok(text)
    }
}
