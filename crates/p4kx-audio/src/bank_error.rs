use std::fmt;

use thiserror::Error;

use crate::HircKind;

/// The four-character tag of a soundbank section, e.g. `BKHD`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionId(pub [u8; 4]);

impl SectionId {
    /// The bank header.
    pub const BKHD: Self = Self(*b"BKHD");
    /// The index of the embedded media.
    pub const DIDX: Self = Self(*b"DIDX");
    /// The embedded media.
    pub const DATA: Self = Self(*b"DATA");
    /// The hierarchy of sound objects, events and actions.
    pub const HIRC: Self = Self(*b"HIRC");
    /// The names of soundbanks by ID.
    pub const STID: Self = Self(*b"STID");

    /// Returns the section ID of the tag `id`.
    #[must_use]
    pub const fn new(id: [u8; 4]) -> Self {
        Self(id)
    }
}

impl fmt::Display for SectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            if byte.is_ascii_graphic() || byte == b' ' {
                write!(f, "{}", char::from(byte))?;
            } else {
                write!(f, "\\x{byte:02x}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for SectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SectionId({self})")
    }
}

/// A failure to read a soundbank; every variant names the section at fault.
#[derive(Debug, Clone, Error, PartialEq, Eq, Hash)]
pub enum BankError {
    /// The data does not start with a `BKHD` section.
    #[error("not a soundbank: the BKHD section is missing")]
    NotSoundbank,
    /// The bank version is older than the oldest layout this reader knows.
    #[error("the soundbank version {version} in BKHD is not supported")]
    UnsupportedVersion {
        /// The version `BKHD` declares.
        version: u32,
    },
    /// The layout of an object is not known for the bank version, so its body is kept raw.
    #[error(
        "the soundbank section HIRC holds an object of type {kind} in version {version}, whose \n         layout is not known"
    )]
    UnsupportedObjectLayout {
        /// The object type.
        kind: HircKind,
        /// The bank version.
        version: u32,
    },
    /// A section ends before a field or an item it declares.
    #[error("the soundbank section {section} is truncated: {len} bytes, {needed} needed")]
    Truncated {
        /// The section.
        section: SectionId,
        /// Bytes available.
        len: u64,
        /// Bytes needed.
        needed: u64,
    },
    /// A section contradicts itself or another section.
    #[error("the soundbank section {section} is invalid: {reason}")]
    InvalidSection {
        /// The section.
        section: SectionId,
        /// What is wrong.
        reason: &'static str,
    },
    /// A section that may appear once appears again.
    #[error("the soundbank section {0} appears twice")]
    DuplicateSection(SectionId),
    /// A `DIDX` entry points outside the `DATA` section.
    #[error(
        "the soundbank section DIDX places WEM {id} at {offset}..{end}, outside the {data_len} \
         bytes of DATA"
    )]
    MediaOutsideData {
        /// The WEM ID.
        id: u32,
        /// Its offset in `DATA`.
        offset: u64,
        /// Its end in `DATA`.
        end: u64,
        /// The length of `DATA`.
        data_len: u64,
    },
}

impl BankError {
    /// Returns the section the error is about, if the data is a soundbank at all.
    #[must_use]
    pub const fn section(&self) -> Option<SectionId> {
        match self {
            Self::NotSoundbank => None,
            Self::UnsupportedVersion { .. } => Some(SectionId::BKHD),
            Self::UnsupportedObjectLayout { .. } => Some(SectionId::HIRC),
            Self::Truncated { section, .. }
            | Self::InvalidSection { section, .. }
            | Self::DuplicateSection(section) => Some(*section),
            Self::MediaOutsideData { .. } => Some(SectionId::DIDX),
        }
    }

    pub(crate) const fn truncated(section: SectionId, len: usize, needed: u64) -> Self {
        Self::Truncated {
            section,
            len: len as u64,
            needed,
        }
    }

    pub(crate) const fn invalid(section: SectionId, reason: &'static str) -> Self {
        Self::InvalidSection { section, reason }
    }
}
