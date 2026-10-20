use p4kx_audio::BankError;
use thiserror::Error;

use super::{MAX_AUDIO_TRIGGERS, MAX_LANGUAGES, MAX_LOOSE_WEMS};

/// Why an ATL definition or soundbank Entry is left out of the audio index.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum AudioFileError {
    /// The Entry is larger than [`MAX_SOUNDBANK_LEN`](super::MAX_SOUNDBANK_LEN) or
    /// [`MAX_ATL_LEN`](super::MAX_ATL_LEN).
    #[error("the Entry holds {size} bytes, more than the limit of {limit} bytes")]
    TooLarge {
        /// Its size.
        size: u64,
        /// The limit for its kind.
        limit: u64,
    },
    /// The Entry's data cannot be read from the Archive.
    #[error("the Entry cannot be read: {reason}")]
    Unreadable {
        /// Why.
        reason: String,
    },
    /// The ATL definition is neither well-formed text XML nor `CryXml`.
    #[error("the ATL definition cannot be read: {reason}")]
    Atl {
        /// Why.
        reason: String,
    },
    /// The soundbank cannot be parsed.
    #[error("the soundbank cannot be parsed: {0}")]
    Soundbank(#[from] BankError),
    /// The index already holds [`MAX_AUDIO_TRIGGERS`] triggers.
    #[error("the audio index already holds {MAX_AUDIO_TRIGGERS} triggers")]
    TooManyTriggers,
    /// The index already holds [`MAX_LOOSE_WEMS`] loose WEMs or [`MAX_LANGUAGES`] languages.
    #[error(
        "the audio index already holds {MAX_LOOSE_WEMS} loose WEMs or {MAX_LANGUAGES} languages"
    )]
    TooManyMedia,
}

impl AudioFileError {
    /// Returns a stable code for the kind of error, e.g. `too_large`, for scripts.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "too_large",
            Self::Unreadable { .. } => "unreadable",
            Self::Atl { .. } => "atl",
            Self::Soundbank(_) => "soundbank",
            Self::TooManyTriggers => "too_many_triggers",
            Self::TooManyMedia => "too_many_media",
        }
    }
}

/// An Entry left out of the audio index, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioIssue {
    pub(crate) path: Box<str>,
    pub(crate) error: AudioFileError,
}

impl AudioIssue {
    /// Returns the path of the Entry.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns why it is left out.
    #[must_use]
    pub const fn error(&self) -> &AudioFileError {
        &self.error
    }
}
