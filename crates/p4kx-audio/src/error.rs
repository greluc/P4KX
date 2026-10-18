use std::io;

use thiserror::Error;

use crate::{Codec, VorbisError};

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
    /// The codec cannot be converted to Ogg yet.
    #[error("converting {0} to Ogg is not supported")]
    UnsupportedOggCodec(Codec),
    /// The Wwise Vorbis header or packet layout is malformed.
    #[error("the Wwise Vorbis data is invalid: {0}")]
    InvalidVorbis(VorbisError),
    /// No codebooks give a Vorbis stream a decoder accepts.
    #[error(
        "the Wwise Vorbis setup fits no codebooks: default: {default}; aoTuV: {aotuv}; inline: {inline}"
    )]
    CodebooksRejected {
        /// Why the default codebook library failed.
        default: VorbisError,
        /// Why the aoTuV codebook library failed.
        aotuv: VorbisError,
        /// Why inline codebooks failed.
        inline: VorbisError,
    },
    /// The WAV is larger than the 4 GiB a RIFF size field can describe.
    #[error("the WAV would exceed 4 GiB")]
    WavTooLarge,
    /// The converted sound could not be written.
    #[error("cannot write the converted sound")]
    Write(#[source] io::Error),
}
