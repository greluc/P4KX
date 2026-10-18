use thiserror::Error;

/// A `global.ini` cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum LocalizationError {
    /// The file is larger than the limit, or its index needs more memory than can be reserved.
    #[error("the global.ini has {size} bytes, more than the limit of {limit} bytes")]
    TooLarge {
        /// The size of the file in bytes.
        size: u64,
        /// The largest size that is read.
        limit: u64,
    },
    /// The file holds more keys than the limit.
    #[error("the global.ini holds more than {limit} keys")]
    TooManyKeys {
        /// The most keys that are read.
        limit: usize,
    },
    /// A line is not UTF-8.
    #[error("line {line} of the global.ini is not UTF-8")]
    InvalidUtf8 {
        /// The line, counted from 1.
        line: usize,
    },
}
