use p4kx_foundation::ByteReader;

use crate::{ByteOrder, Chunk, ChunkError, ChunkType, Container};

const HEADER_LEN: usize = 16;
const TABLE_ENTRY_LEN: usize = 16;
const CRCH_BIG_ENDIAN_FLAG: u16 = 0x8000;

/// A parsed chunk file: its container form and its chunk table over the file's bytes.
#[derive(Debug, Clone)]
pub struct ChunkFile<B> {
    bytes: B,
    container: Container,
    chunks: Vec<Chunk>,
}

impl<B: AsRef<[u8]>> ChunkFile<B> {
    /// Parses the header and chunk table of `bytes`; chunk data is not interpreted.
    ///
    /// # Errors
    ///
    /// Returns [`ChunkError::UnknownSignature`] or [`ChunkError::UnsupportedVersion`] when the
    /// file is no chunk file this crate reads, and [`ChunkError::Truncated`],
    /// [`ChunkError::TableOutOfBounds`] or [`ChunkError::ChunkOutOfBounds`] when it is truncated
    /// or corrupt.
    pub fn parse(bytes: B) -> Result<Self, ChunkError> {
        let data = bytes.as_ref();
        let mut reader = ByteReader::new(data);
        let signature = reader.read_bytes(4)?;
        let container = Container::from_signature(signature).ok_or_else(|| {
            let mut found = [0; 4];
            found.copy_from_slice(signature);
            ChunkError::UnknownSignature { signature: found }
        })?;
        let version = reader.read_u32()?;
        if version != container.version() {
            return Err(ChunkError::UnsupportedVersion { container, version });
        }
        let count = reader.read_u32()?;
        let table_offset = reader.read_u32()?;
        let table = table(data, table_offset, count)?;
        let chunks = match container {
            Container::Ivo => read_ivo(data, table, table_offset)?,
            Container::CrCh => read_crch(data, table)?,
        };
        Ok(Self {
            bytes,
            container,
            chunks,
        })
    }

    /// Returns the container form.
    #[must_use]
    pub const fn container(&self) -> Container {
        self.container
    }

    /// Returns the chunk table in file order.
    #[must_use]
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Returns the data of the chunk at `index` in the table, or `None` when there is no such chunk.
    #[must_use]
    pub fn chunk_data(&self, index: usize) -> Option<&[u8]> {
        let chunk = self.chunks.get(index)?;
        let start = usize::try_from(chunk.offset).ok()?;
        let end = start.checked_add(usize::try_from(chunk.size).ok()?)?;
        self.bytes.as_ref().get(start..end)
    }

    /// Returns the whole file.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

fn table(data: &[u8], offset: u32, count: u32) -> Result<ByteReader<'_>, ChunkError> {
    let len = data.len() as u64;
    let out_of_bounds = || ChunkError::TableOutOfBounds {
        offset: u64::from(offset),
        count,
        len,
    };
    let start = usize::try_from(offset).map_err(|_| out_of_bounds())?;
    if start < HEADER_LEN {
        return Err(out_of_bounds());
    }
    let table_len = u64::from(count) * TABLE_ENTRY_LEN as u64;
    let end = u64::from(offset)
        .checked_add(table_len)
        .filter(|&end| end <= len)
        .and_then(|end| usize::try_from(end).ok())
        .ok_or_else(out_of_bounds)?;
    let bytes = data.get(start..end).ok_or_else(out_of_bounds)?;
    Ok(ByteReader::with_base(bytes, u64::from(offset)))
}

fn read_crch(data: &[u8], mut table: ByteReader<'_>) -> Result<Vec<Chunk>, ChunkError> {
    let len = data.len() as u64;
    let count = table.remaining() / TABLE_ENTRY_LEN;
    let mut chunks = Vec::with_capacity(count);
    for index in 0..count {
        let type_id = table.read_u16()?;
        let raw_version = table.read_u16()?;
        let id = table.read_u32()?;
        let size = u64::from(table.read_u32()?);
        let offset = u64::from(table.read_u32()?);
        if offset.checked_add(size).is_none_or(|end| end > len) {
            return Err(ChunkError::ChunkOutOfBounds {
                index,
                offset,
                size,
                len,
            });
        }
        let byte_order = if raw_version & CRCH_BIG_ENDIAN_FLAG == 0 {
            ByteOrder::Little
        } else {
            ByteOrder::Big
        };
        chunks.push(Chunk {
            kind: ChunkType::new(Container::CrCh, u32::from(type_id)),
            version: u32::from(raw_version & !CRCH_BIG_ENDIAN_FLAG),
            id: Some(id),
            offset,
            size,
            byte_order,
        });
    }
    Ok(chunks)
}

fn read_ivo(
    data: &[u8],
    mut table: ByteReader<'_>,
    table_offset: u32,
) -> Result<Vec<Chunk>, ChunkError> {
    let len = data.len() as u64;
    let count = table.remaining() / TABLE_ENTRY_LEN;
    let mut chunks = Vec::with_capacity(count);
    for index in 0..count {
        let type_id = table.read_u32()?;
        let version = table.read_u32()?;
        let offset = table.read_u64()?;
        if offset > len {
            return Err(ChunkError::ChunkOutOfBounds {
                index,
                offset,
                size: 0,
                len,
            });
        }
        chunks.push(Chunk {
            kind: ChunkType::new(Container::Ivo, type_id),
            version,
            id: None,
            offset,
            size: 0,
            byte_order: ByteOrder::Little,
        });
    }
    let mut order: Vec<(u64, usize)> = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| (chunk.offset, index))
        .collect();
    order.sort_unstable();
    let table_start = u64::from(table_offset);
    for (position, &(offset, index)) in order.iter().enumerate() {
        let next = order.get(position + 1).map_or(len, |&(next, _)| next);
        let end = if table_start > offset && table_start < next {
            table_start
        } else {
            next
        };
        if let Some(chunk) = chunks.get_mut(index) {
            chunk.size = end.saturating_sub(offset);
        }
    }
    Ok(chunks)
}
