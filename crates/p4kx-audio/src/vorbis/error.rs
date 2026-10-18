use thiserror::Error;

/// A reason a Wwise Vorbis WEM cannot be rebuilt as Ogg Vorbis.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum VorbisError {
    /// The Wwise Vorbis header has a layout this version does not read.
    #[error("the Wwise Vorbis header of {len} bytes is not supported")]
    UnsupportedHeader {
        /// Length of the header in bytes.
        len: usize,
    },
    /// The block sizes are outside 64 to 8192 or the short one exceeds the long one.
    #[error("invalid block sizes 2^{short} and 2^{long}")]
    BlockSizes {
        /// Exponent of the short block size.
        short: u8,
        /// Exponent of the long block size.
        long: u8,
    },
    /// Vorbis carries at most 255 channels.
    #[error("{0} channels are more than Vorbis carries")]
    TooManyChannels(u16),
    /// A packet header or payload lies outside the `data` chunk.
    #[error("the packet at data offset {offset} runs past the data chunk")]
    PacketOutOfBounds {
        /// Offset of the packet header in the `data` chunk.
        offset: u64,
    },
    /// The setup packet ends before its last field.
    #[error("the setup packet ends early")]
    SetupTruncated,
    /// The rebuilt setup did not use exactly the bytes of the setup packet.
    #[error("the setup used {used} of its {len} bytes")]
    SetupLength {
        /// Bytes the rebuilt setup read.
        used: usize,
        /// Bytes of the setup packet.
        len: usize,
    },
    /// The codebook library has no codebook with this id.
    #[error("the codebook library has no codebook {0}")]
    UnknownCodebook(u32),
    /// A codebook is malformed.
    #[error("invalid codebook: {0}")]
    InvalidCodebook(&'static str),
    /// A floor, residue, mapping or mode of the setup is malformed.
    #[error("invalid setup: {0}")]
    InvalidSetup(&'static str),
    /// An audio packet is malformed.
    #[error("invalid audio packet at data offset {offset}: {reason}")]
    InvalidAudioPacket {
        /// Offset of the packet header in the `data` chunk.
        offset: u64,
        /// What is wrong.
        reason: &'static str,
    },
    /// The Vorbis decoder fails on an audio packet while decoding the sound to PCM.
    #[error("the Vorbis decoder fails on the audio packet at data offset {offset}: {reason}")]
    Undecodable {
        /// Offset of the packet header in the `data` chunk.
        offset: u64,
        /// What the decoder reports.
        reason: String,
    },
    /// A Vorbis decoder rejects the rebuilt stream.
    #[error("a Vorbis decoder rejects the rebuilt stream: {0}")]
    Rejected(String),
}
