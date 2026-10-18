use std::fmt;

/// The compression method of an Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compression {
    /// Data is stored without compression (ZIP method 0).
    Store,
    /// Data is raw deflate (ZIP method 8).
    Deflate,
    /// Data is zstd (CIG method 100).
    Zstd,
    /// Any other ZIP method id.
    Other(u16),
}

impl Compression {
    /// Maps a ZIP compression method id.
    #[must_use]
    pub const fn from_method(method: u16) -> Self {
        match method {
            0 => Self::Store,
            8 => Self::Deflate,
            100 => Self::Zstd,
            other => Self::Other(other),
        }
    }
}

impl fmt::Display for Compression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store => f.write_str("store"),
            Self::Deflate => f.write_str("deflate"),
            Self::Zstd => f.write_str("zstd"),
            Self::Other(method) => write!(f, "method-{method}"),
        }
    }
}
