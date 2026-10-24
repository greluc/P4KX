// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

/// A `global.ini` cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum LocalizationError {
    /// The file is larger than the limit.
    #[error("the global.ini has {size} bytes, more than the limit of {limit} bytes")]
    TooLarge {
        /// The size of the file in bytes.
        size: u64,
        /// The largest size that is read.
        limit: u64,
    },
    /// The memory to decode the file to UTF-8 cannot be reserved.
    #[error("cannot reserve the memory to decode the {size} bytes of the global.ini")]
    DecodeOutOfMemory {
        /// The size of the file in bytes.
        size: u64,
    },
    /// The memory for the index of the file's keys cannot be reserved.
    #[error("cannot reserve the memory to index {keys} keys of the global.ini")]
    OutOfMemory {
        /// The number of keys the reservation was for.
        keys: usize,
    },
    /// The file holds more keys than the limit.
    #[error("the global.ini holds more than {limit} keys")]
    TooManyKeys {
        /// The most keys that are read.
        limit: usize,
    },
}
