use thiserror::Error;

/// Why a Comparison did not produce a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ComparisonError {
    /// No level was selected.
    #[error("no Comparison level is selected")]
    NoLevel,
    /// The Comparison was cancelled through its [`CancelToken`](crate::CancelToken).
    #[error("the Comparison was cancelled")]
    Cancelled,
}

/// Why the line diff of an Entry pair could not be produced.
#[derive(Debug, Error)]
pub enum LineDiffError {
    /// Neither Archive has an Entry with the path.
    #[error("no Entry named {path} in either Archive")]
    UnknownEntry {
        /// The requested path.
        path: String,
    },
    /// The data of an Entry cannot be opened.
    #[error(transparent)]
    Archive(#[from] crate::ArchiveError),
    /// The data of an Entry cannot be read or decompressed.
    #[error("cannot read the Entry {path}")]
    Read {
        /// The Entry path.
        path: String,
        /// What failed.
        #[source]
        source: std::io::Error,
    },
    /// The `CryXml` of an Entry is truncated or corrupt.
    #[error("cannot convert the CryXml Entry {path}")]
    CryXml {
        /// The Entry path.
        path: String,
        /// What is wrong with it.
        #[source]
        source: p4kx_cryxml::CryXmlError,
    },
}
