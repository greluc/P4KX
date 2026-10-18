use std::fmt;

/// The two forms of the chunk container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Container {
    /// The newer Star Citizen form, signature `#ivo`, version `0x900`, with 32-bit hash type IDs.
    Ivo,
    /// The older `CryEngine` form, signature `CrCh`, version `0x746`, with 16-bit type IDs.
    CrCh,
}

impl Container {
    /// Returns the four signature bytes at the start of the file.
    #[must_use]
    pub const fn signature(self) -> &'static [u8; 4] {
        match self {
            Self::Ivo => b"#ivo",
            Self::CrCh => b"CrCh",
        }
    }

    /// Returns the container version this crate reads.
    #[must_use]
    pub const fn version(self) -> u32 {
        match self {
            Self::Ivo => 0x900,
            Self::CrCh => 0x746,
        }
    }

    pub(crate) fn from_signature(signature: &[u8]) -> Option<Self> {
        [Self::Ivo, Self::CrCh]
            .into_iter()
            .find(|container| container.signature() == signature)
    }
}

impl fmt::Display for Container {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.pad(match self {
            Self::Ivo => "#ivo",
            Self::CrCh => "CrCh",
        })
    }
}

/// The byte order of a chunk's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteOrder {
    /// Least significant byte first.
    Little,
    /// Most significant byte first.
    Big,
}

impl ByteOrder {
    /// Returns `little` or `big`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Little => "little",
            Self::Big => "big",
        }
    }
}

impl fmt::Display for ByteOrder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.pad(self.name())
    }
}
