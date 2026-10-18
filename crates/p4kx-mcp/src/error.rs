use std::error::Error;
use std::path::PathBuf;

use p4kx_extraction::ArchiveError;
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
    /// The settings file with the manual Channel entries cannot be read.
    #[error("cannot read the settings")]
    Settings(#[from] SettingsError),
    /// The worker running the call ended without a result.
    #[error("the tool call stopped unexpectedly")]
    Stopped,
}

impl ToolError {
    /// Returns the message with every cause, `: ` separated.
    pub(crate) fn message(&self) -> String {
        let mut message = self.to_string();
        let mut source = self.source();
        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }
        message
    }
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
