use std::io::{self, Read};

use crate::header::MAX_HEADER_LEN;
use crate::merge::{checked_part_len, read_prefix};
use crate::{DdsHeader, HdrImage, RgbaImage, SiblingReader, TextureError, decode, decode_hdr};

const INITIAL_CAPACITY: u64 = 16 * 1024 * 1024;

/// The data of one mip of a texture with a single image per mip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mip {
    header: DdsHeader,
    level: u32,
    data: Vec<u8>,
}

impl Mip {
    /// Returns the header of the texture.
    #[must_use]
    pub const fn header(&self) -> &DdsHeader {
        &self.header
    }

    /// Returns the mip level, 0 being the largest.
    #[must_use]
    pub const fn level(&self) -> u32 {
        self.level
    }

    /// Returns the width of the mip in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        side(self.header.width(), self.level)
    }

    /// Returns the height of the mip in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        side(self.header.height(), self.level)
    }

    /// Returns the mip's data in the texture's format.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Decodes the mip to 8-bit RGBA.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`decode`].
    pub fn decode(&self) -> Result<RgbaImage, TextureError> {
        decode(
            self.header.format(),
            self.width(),
            self.height(),
            &self.data,
        )
    }
    /// Decodes the mip of a high-range texture to floating-point RGB without clipping.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`decode_hdr`].
    pub fn decode_hdr(&self) -> Result<HdrImage, TextureError> {
        decode_hdr(
            self.header.format(),
            self.width(),
            self.height(),
            &self.data,
        )
    }
}

const fn side(full: u32, level: u32) -> u32 {
    match full.checked_shr(level) {
        Some(side) if side > 0 => side,
        _ => 1,
    }
}

/// Reads mip `level`, 0 being the largest, of the header Entry `header` of `header_size` bytes:
/// from its sibling part when the mip is split off, otherwise from the header's tail.
///
/// # Errors
///
/// Returns [`TextureError::NoSuchMip`] when the level does not exist,
/// [`TextureError::UnsupportedImage`] when the texture has several images per mip or several
/// slices, the errors of [`DdsHeader::part_count`], and a part or header error when the mip's
/// data is missing, truncated, oversized or unreadable.
pub fn read_mip<R, S>(
    mut header: R,
    header_size: u64,
    siblings: &S,
    level: u32,
) -> Result<Mip, TextureError>
where
    R: Read,
    S: SiblingReader + ?Sized,
{
    let mut prefix = [0; MAX_HEADER_LEN];
    let limit =
        usize::try_from(header_size).map_or(MAX_HEADER_LEN, |size| size.min(MAX_HEADER_LEN));
    let prefix_len = read_prefix(&mut header, prefix.get_mut(..limit).unwrap_or_default())?;
    let start = prefix.get(..prefix_len).unwrap_or_default();
    let dds = DdsHeader::parse(start)?;
    if level >= dds.mip_count() {
        return Err(TextureError::NoSuchMip {
            level,
            mip_count: dds.mip_count(),
        });
    }
    let images = dds.surfaces().saturating_mul(dds.depth());
    if images > 1 {
        return Err(TextureError::UnsupportedImage { images });
    }
    let parts = dds.part_count(header_size)?;
    let size = dds
        .mip_size(level)
        .ok_or_else(|| TextureError::UnsupportedFormat(dds.format()))?;
    let data = if level < parts {
        read_from_part(&dds, parts, level, size, siblings)?
    } else {
        let tail = start.get(dds.header_len()..).unwrap_or_default();
        read_from_tail(&dds, parts, level, size, tail.chain(header), header_size)?
    };
    Ok(Mip {
        header: dds,
        level,
        data,
    })
}

fn read_from_part<S>(
    dds: &DdsHeader,
    parts: u32,
    level: u32,
    size: u64,
    siblings: &S,
) -> Result<Vec<u8>, TextureError>
where
    S: SiblingReader + ?Sized,
{
    let index = parts - level;
    checked_part_len(dds, parts, index, siblings)?;
    let reader = siblings
        .open_part(index)
        .map_err(|source| TextureError::ReadPart { index, source })?;
    let data =
        read_up_to(reader, size).map_err(|source| TextureError::ReadPart { index, source })?;
    if (data.len() as u64) < size {
        return Err(TextureError::TruncatedPart {
            index,
            expected: size,
            actual: data.len() as u64,
        });
    }
    Ok(data)
}

fn read_from_tail(
    dds: &DdsHeader,
    parts: u32,
    level: u32,
    size: u64,
    mut tail: impl Read,
    header_size: u64,
) -> Result<Vec<u8>, TextureError> {
    let skip: u64 = (parts..level)
        .filter_map(|smaller| dds.mip_size(smaller))
        .sum();
    let skipped = io::copy(&mut tail.by_ref().take(skip), &mut io::sink())
        .map_err(TextureError::ReadHeader)?;
    let data = read_up_to(tail, size).map_err(TextureError::ReadHeader)?;
    if skipped < skip || (data.len() as u64) < size {
        return Err(TextureError::TruncatedHeader {
            len: dds.header_len() as u64 + skipped + data.len() as u64,
            needed: header_size,
        });
    }
    Ok(data)
}

fn read_up_to(reader: impl Read, size: u64) -> io::Result<Vec<u8>> {
    let capacity = usize::try_from(size.min(INITIAL_CAPACITY)).unwrap_or_default();
    let mut data = Vec::with_capacity(capacity);
    reader.take(size).read_to_end(&mut data)?;
    Ok(data)
}
