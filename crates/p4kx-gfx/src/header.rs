use std::fmt;
use std::io::{self, Read};

use flate2::read::ZlibDecoder;
use lzma_rs::decompress::{Options, UnpackedSize};
use p4kx_foundation::ParseError;

use crate::GfxError;
use crate::geometry::Rect;
use crate::reader::TagReader;

/// The length of the file header before the movie header: signature, version and length.
pub(crate) const FILE_HEADER_LEN: usize = 8;

/// The largest movie decompressed into memory, far above the largest movie of a real Archive.
pub const MAX_MOVIE_LEN: u64 = 64 << 20;

const LZMA_PROPERTIES_START: usize = 12;

/// The signature a movie starts with, naming its container and compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signature {
    /// An uncompressed SWF movie, `FWS`.
    Fws,
    /// A zlib-compressed SWF movie, `CWS`.
    Cws,
    /// An LZMA-compressed SWF movie, `ZWS`.
    Zws,
    /// An uncompressed Scaleform GFx movie, `GFX`.
    Gfx,
    /// A zlib-compressed Scaleform GFx movie, `CFX`.
    Cfx,
}

/// How the movie after the file header is compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compression {
    /// Not compressed.
    None,
    /// zlib.
    Zlib,
    /// LZMA.
    Lzma,
}

impl Signature {
    const fn parse(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"FWS" => Some(Self::Fws),
            b"CWS" => Some(Self::Cws),
            b"ZWS" => Some(Self::Zws),
            b"GFX" => Some(Self::Gfx),
            b"CFX" => Some(Self::Cfx),
            _ => None,
        }
    }

    /// Returns the three signature characters.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fws => "FWS",
            Self::Cws => "CWS",
            Self::Zws => "ZWS",
            Self::Gfx => "GFX",
            Self::Cfx => "CFX",
        }
    }

    /// Returns whether the movie is a Scaleform GFx movie rather than a plain SWF movie.
    #[must_use]
    pub const fn is_gfx(self) -> bool {
        matches!(self, Self::Gfx | Self::Cfx)
    }

    /// Returns how the movie after the file header is compressed.
    #[must_use]
    pub const fn compression(self) -> Compression {
        match self {
            Self::Fws | Self::Gfx => Compression::None,
            Self::Cws | Self::Cfx => Compression::Zlib,
            Self::Zws => Compression::Lzma,
        }
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The file and movie header of a GFx or SWF movie.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Header {
    /// The signature.
    pub signature: Signature,
    /// The SWF or GFx version.
    pub version: u8,
    /// The uncompressed length the file header declares, file header included.
    pub declared_len: u32,
    /// The stage in twips.
    pub frame_size: Rect,
    /// The frames per second.
    pub frame_rate: f32,
    /// The frames of the main timeline the header declares.
    pub frame_count: u16,
}

/// Reads the file header, decompresses the movie and reads the movie header.
///
/// Returns the header, the whole uncompressed movie with the file header in front, and the
/// offset of the first tag.
pub(crate) fn read_movie(bytes: Vec<u8>) -> Result<(Header, Vec<u8>, usize), GfxError> {
    let Some(file_header) = bytes.get(..FILE_HEADER_LEN) else {
        return Err(GfxError::Parse(ParseError::UnexpectedEnd {
            offset: 0,
            needed: FILE_HEADER_LEN as u64,
            available: bytes.len() as u64,
        }));
    };
    let signature = file_header
        .get(..3)
        .and_then(Signature::parse)
        .ok_or_else(|| GfxError::Signature {
            found: file_header.get(..3).unwrap_or_default().to_vec(),
        })?;
    let mut reader = TagReader::new(file_header, 0);
    reader.bytes(3)?;
    let version = reader.u8()?;
    let declared_len = reader.u32()?;
    let declared = usize::try_from(declared_len).unwrap_or(usize::MAX);
    if declared < FILE_HEADER_LEN {
        return Err(GfxError::DeclaredLength {
            declared: declared_len,
        });
    }
    let data = match signature.compression() {
        Compression::None => {
            let mut bytes = bytes;
            bytes.truncate(declared);
            bytes
        }
        compression => decompress(&bytes, declared, compression)?,
    };
    let body = data.get(FILE_HEADER_LEN..).unwrap_or_default();
    let mut reader = TagReader::new(body, FILE_HEADER_LEN);
    let frame_size = reader.rect()?;
    let rate = reader.u16()?;
    let frame_count = reader.u16()?;
    let header = Header {
        signature,
        version,
        declared_len,
        frame_size,
        frame_rate: f32::from(rate >> 8) + f32::from(rate & 0xFF) / 256.0,
        frame_count,
    };
    let tags = reader.offset();
    Ok((header, data, tags))
}

fn decompress(
    bytes: &[u8],
    declared: usize,
    compression: Compression,
) -> Result<Vec<u8>, GfxError> {
    if declared as u64 > MAX_MOVIE_LEN {
        return Err(GfxError::TooLarge {
            limit: MAX_MOVIE_LEN,
        });
    }
    let body_len = declared - FILE_HEADER_LEN;
    let capacity = declared.min(bytes.len().saturating_mul(4));
    let mut data = Vec::with_capacity(capacity);
    data.extend_from_slice(bytes.get(..FILE_HEADER_LEN).unwrap_or_default());
    let compressed = bytes.get(FILE_HEADER_LEN..).unwrap_or_default();
    match compression {
        Compression::None => data.extend_from_slice(compressed),
        Compression::Zlib => {
            ZlibDecoder::new(compressed)
                .take(body_len as u64)
                .read_to_end(&mut data)
                .map_err(|error| decompress_error(&error))?;
        }
        Compression::Lzma => {
            let mut input = bytes.get(LZMA_PROPERTIES_START..).unwrap_or_default();
            let options = Options {
                unpacked_size: UnpackedSize::UseProvided(Some(body_len as u64)),
                memlimit: Some(body_len.max(1)),
                allow_incomplete: false,
            };
            lzma_rs::lzma_decompress_with_options(&mut input, &mut data, &options).map_err(
                |error| GfxError::Decompress {
                    message: error.to_string(),
                },
            )?;
        }
    }
    Ok(data)
}

fn decompress_error(error: &io::Error) -> GfxError {
    GfxError::Decompress {
        message: error.to_string(),
    }
}
