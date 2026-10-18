use crate::bytes::{put_u16, put_u32, put_u64};

const HEADER_LEN: usize = 16;
const TABLE_ENTRY_LEN: usize = 16;
const IVO_ALIGNMENT: usize = 8;
const CRCH_BIG_ENDIAN_FLAG: u16 = 0x8000;

/// The container form a [`TestChunkFile`] is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestChunkContainer {
    /// `#ivo` version `0x900`: 32-bit type IDs, 64-bit offsets, no sizes, data 8-byte aligned.
    Ivo,
    /// `CrCh` version `0x746`: 16-bit type IDs and versions, chunk IDs, 32-bit sizes and offsets.
    CrCh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestChunk {
    type_id: u32,
    version: u32,
    big_endian: bool,
    data: Vec<u8>,
}

/// A synthetic `CryEngine` chunk file: header, chunk table, then the chunks' data in order, or the
/// table after the data with [`Self::table_at_end`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestChunkFile {
    container: TestChunkContainer,
    chunks: Vec<TestChunk>,
    table_at_end: bool,
}

impl TestChunkFile {
    /// Starts an empty `#ivo` file.
    #[must_use]
    pub const fn ivo() -> Self {
        Self {
            container: TestChunkContainer::Ivo,
            chunks: Vec::new(),
            table_at_end: false,
        }
    }

    /// Starts an empty `CrCh` file; its chunks get the IDs 1, 2, … in order.
    #[must_use]
    pub const fn crch() -> Self {
        Self {
            container: TestChunkContainer::CrCh,
            chunks: Vec::new(),
            table_at_end: false,
        }
    }

    /// Adds a little-endian chunk; in `CrCh`, `type_id` and `version` are cut to 16 bits.
    #[must_use]
    pub fn chunk(self, type_id: u32, version: u32, data: impl Into<Vec<u8>>) -> Self {
        self.push(type_id, version, false, data.into())
    }

    /// Adds a chunk flagged as big-endian, which only `CrCh` can express; `data` is written as is.
    #[must_use]
    pub fn big_endian_chunk(self, type_id: u32, version: u32, data: impl Into<Vec<u8>>) -> Self {
        self.push(type_id, version, true, data.into())
    }

    /// Writes the chunk table after the chunks' data instead of before it.
    #[must_use]
    pub const fn table_at_end(mut self) -> Self {
        self.table_at_end = true;
        self
    }

    fn push(mut self, type_id: u32, version: u32, big_endian: bool, data: Vec<u8>) -> Self {
        self.chunks.push(TestChunk {
            type_id,
            version,
            big_endian,
            data,
        });
        self
    }

    /// Returns the offset each chunk's data is written at, in order.
    #[must_use]
    pub fn offsets(&self) -> Vec<u64> {
        let mut offset = HEADER_LEN
            + if self.table_at_end {
                0
            } else {
                self.table_len()
            };
        self.chunks
            .iter()
            .map(|chunk| {
                offset = self.align(offset);
                let start = offset;
                offset += chunk.data.len();
                start as u64
            })
            .collect()
    }

    /// Returns the bytes of the file.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let offsets = self.offsets();
        let mut out = Vec::new();
        let (signature, version) = match self.container {
            TestChunkContainer::Ivo => (b"#ivo", 0x900),
            TestChunkContainer::CrCh => (b"CrCh", 0x746),
        };
        out.extend_from_slice(signature);
        put_u32(&mut out, version);
        put_u32(
            &mut out,
            u32::try_from(self.chunks.len()).unwrap_or(u32::MAX),
        );
        let data_end = self
            .chunks
            .last()
            .zip(offsets.last())
            .map_or(HEADER_LEN, |(chunk, &offset)| {
                usize::try_from(offset).unwrap_or(usize::MAX) + chunk.data.len()
            });
        let table_offset = if self.table_at_end {
            self.align(data_end)
        } else {
            HEADER_LEN
        };
        put_u32(&mut out, u32::try_from(table_offset).unwrap_or(u32::MAX));
        let mut table = Vec::with_capacity(self.table_len());
        for ((index, chunk), &offset) in self.chunks.iter().enumerate().zip(&offsets) {
            self.put_entry(&mut table, index, chunk, offset);
        }
        if !self.table_at_end {
            out.extend_from_slice(&table);
        }
        for (chunk, &offset) in self.chunks.iter().zip(&offsets) {
            out.resize(usize::try_from(offset).unwrap_or(usize::MAX), 0);
            out.extend_from_slice(&chunk.data);
        }
        if self.table_at_end {
            out.resize(table_offset, 0);
            out.extend_from_slice(&table);
        }
        out
    }

    const fn table_len(&self) -> usize {
        self.chunks.len() * TABLE_ENTRY_LEN
    }

    fn put_entry(&self, out: &mut Vec<u8>, index: usize, chunk: &TestChunk, offset: u64) {
        match self.container {
            TestChunkContainer::Ivo => {
                put_u32(out, chunk.type_id);
                put_u32(out, chunk.version);
                put_u64(out, offset);
            }
            TestChunkContainer::CrCh => {
                let flag = if chunk.big_endian {
                    CRCH_BIG_ENDIAN_FLAG
                } else {
                    0
                };
                put_u16(out, low_u16(chunk.type_id));
                put_u16(out, (low_u16(chunk.version) & !CRCH_BIG_ENDIAN_FLAG) | flag);
                put_u32(out, u32::try_from(index + 1).unwrap_or(u32::MAX));
                put_u32(out, u32::try_from(chunk.data.len()).unwrap_or(u32::MAX));
                put_u32(out, u32::try_from(offset).unwrap_or(u32::MAX));
            }
        }
    }

    const fn align(&self, offset: usize) -> usize {
        match self.container {
            TestChunkContainer::Ivo => offset.next_multiple_of(IVO_ALIGNMENT),
            TestChunkContainer::CrCh => offset,
        }
    }
}

const fn low_u16(value: u32) -> u16 {
    let [low, high, ..] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}
