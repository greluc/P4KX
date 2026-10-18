use std::io::{self, Read, Write};

use crate::header::MAX_HEADER_LEN;
use crate::{DdsHeader, TextureError};

const COPY_BUFFER_LEN: usize = 64 * 1024;

/// Gives access to the sibling parts `.dds.1` … `.dds.N` of a header Entry, wherever they are
/// stored.
pub trait SiblingReader {
    /// The reader of one part's data.
    type Part<'a>: Read
    where
        Self: 'a;

    /// Returns the length in bytes of part `index`, or `None` when the part does not exist.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of looking the part up.
    fn part_len(&self, index: u32) -> io::Result<Option<u64>>;

    /// Opens the data of part `index`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of opening the part.
    fn open_part(&self, index: u32) -> io::Result<Self::Part<'_>>;
}

/// What a merge wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeReport {
    /// The header of the merged texture.
    pub header: DdsHeader,
    /// Number of sibling parts read.
    pub parts: u32,
    /// Size of the merged DDS in bytes.
    pub size: u64,
}

/// Writes the header Entry `header` of `header_size` bytes and its sibling parts as one standard
/// DDS file to `out`: the header, then every mip from the largest to the smallest.
///
/// Every part's length is checked before anything is written; the data is streamed.
///
/// # Errors
///
/// Returns a [`TextureError`] when the header is not a DDS header or its format's size is
/// unknown, a part is missing, truncated or oversized, the header's tail is not a run of whole
/// mips, a split texture has several images per mip, or reading or writing fails.
pub fn merge<R, S, W>(
    mut header: R,
    header_size: u64,
    siblings: &S,
    out: &mut W,
) -> Result<MergeReport, TextureError>
where
    R: Read,
    S: SiblingReader + ?Sized,
    W: Write + ?Sized,
{
    let mut prefix = [0; MAX_HEADER_LEN];
    let limit =
        usize::try_from(header_size).map_or(MAX_HEADER_LEN, |size| size.min(MAX_HEADER_LEN));
    let prefix_len = read_prefix(&mut header, prefix.get_mut(..limit).unwrap_or_default())?;
    let start = prefix.get(..prefix_len).unwrap_or_default();
    let dds = DdsHeader::parse(start)?;
    let parts = dds.part_count(header_size)?;
    if parts > 0 && dds.surfaces() > 1 {
        return Err(TextureError::UnsupportedLayout {
            surfaces: dds.surfaces(),
        });
    }
    let sizes = part_sizes(&dds, parts, siblings)?;
    let (own_header, own_tail) = start
        .split_at_checked(dds.header_len())
        .unwrap_or((start, &[]));
    out.write_all(own_header).map_err(TextureError::Write)?;
    let mut buffer = vec![0; COPY_BUFFER_LEN];
    for (index, expected) in (1..=parts).rev().zip(sizes.into_iter().rev()) {
        let reader = siblings
            .open_part(index)
            .map_err(|source| TextureError::ReadPart { index, source })?;
        let copied = copy(reader, expected, out, &mut buffer).map_err(|error| {
            error.into_texture_error(|source| TextureError::ReadPart { index, source })
        })?;
        if copied < expected {
            return Err(TextureError::TruncatedPart {
                index,
                expected,
                actual: copied,
            });
        }
    }
    out.write_all(own_tail).map_err(TextureError::Write)?;
    let rest = header_size.saturating_sub(prefix_len as u64);
    let copied = copy(header, rest, out, &mut buffer)
        .map_err(|error| error.into_texture_error(TextureError::ReadHeader))?;
    if copied < rest {
        return Err(TextureError::TruncatedHeader {
            len: prefix_len as u64 + copied,
            needed: header_size,
        });
    }
    out.flush().map_err(TextureError::Write)?;
    let merged_parts: u64 = (1..=parts)
        .filter_map(|index| dds.mip_size(parts - index))
        .sum();
    Ok(MergeReport {
        header: dds,
        parts,
        size: header_size.saturating_add(merged_parts),
    })
}

fn part_sizes<S>(dds: &DdsHeader, parts: u32, siblings: &S) -> Result<Vec<u64>, TextureError>
where
    S: SiblingReader + ?Sized,
{
    (1..=parts)
        .map(|index| {
            let expected = dds
                .mip_size(parts - index)
                .ok_or_else(|| TextureError::UnsupportedFormat(dds.format()))?;
            let actual = siblings
                .part_len(index)
                .map_err(|source| TextureError::ReadPart { index, source })?
                .ok_or(TextureError::MissingPart { index })?;
            match actual.cmp(&expected) {
                std::cmp::Ordering::Less => Err(TextureError::TruncatedPart {
                    index,
                    expected,
                    actual,
                }),
                std::cmp::Ordering::Greater => Err(TextureError::OversizedPart {
                    index,
                    expected,
                    actual,
                }),
                std::cmp::Ordering::Equal => Ok(expected),
            }
        })
        .collect()
}

fn read_prefix(header: &mut impl Read, prefix: &mut [u8]) -> Result<usize, TextureError> {
    let mut filled = 0;
    while let Some(free) = prefix.get_mut(filled..).filter(|free| !free.is_empty()) {
        match header.read(free) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(TextureError::ReadHeader(error)),
        }
    }
    Ok(filled)
}

enum CopyError {
    Read(io::Error),
    Write(io::Error),
}

impl CopyError {
    fn into_texture_error(self, read: impl FnOnce(io::Error) -> TextureError) -> TextureError {
        match self {
            Self::Read(error) => read(error),
            Self::Write(error) => TextureError::Write(error),
        }
    }
}

fn copy<W: Write + ?Sized>(
    reader: impl Read,
    limit: u64,
    out: &mut W,
    buffer: &mut [u8],
) -> Result<u64, CopyError> {
    let mut reader = reader.take(limit);
    let mut copied = 0;
    loop {
        let count = match reader.read(buffer) {
            Ok(0) => return Ok(copied),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(CopyError::Read(error)),
        };
        out.write_all(buffer.get(..count).unwrap_or_default())
            .map_err(CopyError::Write)?;
        copied += count as u64;
    }
}
