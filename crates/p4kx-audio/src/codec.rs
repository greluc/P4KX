use std::fmt;

/// The codec of a WEM, detected from the format tag of its `fmt` chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codec {
    /// Uncompressed 16-bit PCM, tags `0x0001` and `0xFFFE`.
    Pcm,
    /// IMA ADPCM, tags `0x0002`, `0x0011` and `0x0069`.
    ImaAdpcm,
    /// Wwise Vorbis with stripped headers, tag `0xFFFF`.
    Vorbis,
    /// Opus, tags `0x3039`, `0x3040` and `0x3041`.
    Opus,
    /// Platinum ADPCM, tag `0x8311`.
    PtAdpcm,
    /// Xbox WMA, tags `0x0161` and `0x0162`.
    Xwma,
    /// Xbox XMA2, tags `0x0165` and `0x0166`.
    Xma2,
    /// AAC, tag `0xAAC0`.
    Aac,
    /// Nintendo DSP ADPCM, tag `0xFFF0`.
    Dsp,
    /// Sony HEVAG, tag `0xFFFB`.
    Hevag,
    /// Sony ATRAC9, tag `0xFFFC`.
    Atrac9,
    /// A format tag Wwise is not known to write.
    Unknown(u16),
}

impl Codec {
    /// Returns the codec a Wwise `fmt` chunk's format tag stands for.
    #[must_use]
    pub const fn from_tag(tag: u16) -> Self {
        match tag {
            0x0001 | 0xFFFE => Self::Pcm,
            0x0002 | 0x0011 | 0x0069 => Self::ImaAdpcm,
            0xFFFF => Self::Vorbis,
            0x3039 | 0x3040 | 0x3041 => Self::Opus,
            0x8311 => Self::PtAdpcm,
            0x0161 | 0x0162 => Self::Xwma,
            0x0165 | 0x0166 => Self::Xma2,
            0xAAC0 => Self::Aac,
            0xFFF0 => Self::Dsp,
            0xFFFB => Self::Hevag,
            0xFFFC => Self::Atrac9,
            other => Self::Unknown(other),
        }
    }

    /// Returns a stable lower-case identifier for machine-readable output, e.g. `vorbis`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Pcm => "pcm",
            Self::ImaAdpcm => "ima_adpcm",
            Self::Vorbis => "vorbis",
            Self::Opus => "opus",
            Self::PtAdpcm => "ptadpcm",
            Self::Xwma => "xwma",
            Self::Xma2 => "xma2",
            Self::Aac => "aac",
            Self::Dsp => "dsp",
            Self::Hevag => "hevag",
            Self::Atrac9 => "atrac9",
            Self::Unknown(_) => "unknown",
        }
    }
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Pcm => "PCM",
            Self::ImaAdpcm => "IMA ADPCM",
            Self::Vorbis => "Wwise Vorbis",
            Self::Opus => "Opus",
            Self::PtAdpcm => "PTADPCM",
            Self::Xwma => "XWMA",
            Self::Xma2 => "XMA2",
            Self::Aac => "AAC",
            Self::Dsp => "DSP ADPCM",
            Self::Hevag => "HEVAG",
            Self::Atrac9 => "ATRAC9",
            Self::Unknown(tag) => return write!(f, "unknown codec {tag:#06x}"),
        };
        f.write_str(name)
    }
}
