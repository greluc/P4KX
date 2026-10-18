use std::io;

use thiserror::Error;

use crate::Codec;

/// A failure to read or convert a WEM.
#[derive(Debug, Error)]
pub enum AudioError {
    /// The data is neither a RIFF nor a RIFX WAVE file.
    #[error("not a WEM: the RIFF/RIFX WAVE header is missing")]
    NotWem,
    /// The data is Wwise plug-in media, e.g. a convolution reverb's impulse response.
    #[error("not a sound: Wwise plug-in media")]
    PluginMedia,
    /// The data ends before a header field or a chunk it declares.
    #[error("the WEM is truncated: {len} bytes, {needed} needed")]
    Truncated {
        /// Bytes available.
        len: u64,
        /// Bytes needed.
        needed: u64,
    },
    /// A chunk every WEM needs is missing.
    #[error("the WEM has no `{0}` chunk")]
    MissingChunk(&'static str),
    /// The `fmt` chunk is too short or contradicts itself.
    #[error("the WEM format is invalid: {0}")]
    InvalidFormat(&'static str),
    /// The codec has no decoder to convert it to WAV yet.
    #[error("converting {0} to WAV is not supported")]
    UnsupportedCodec(Codec),
    /// The WAV is larger than the 4 GiB a RIFF size field can describe.
    #[error("the WAV would exceed 4 GiB")]
    WavTooLarge,
    /// The WAV could not be written.
    #[error("cannot write the WAV")]
    Write(#[source] io::Error),
}
