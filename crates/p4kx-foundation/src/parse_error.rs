use thiserror::Error;

/// A failure to read binary data, located by its offset in the input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// The input ended before a value could be read completely.
    #[error("unexpected end of data at offset {offset}: needed {needed} bytes, {available} left")]
    UnexpectedEnd {
        /// Offset at which the read started.
        offset: u64,
        /// Number of bytes the read required.
        needed: u64,
        /// Number of bytes that were left.
        available: u64,
    },
}
