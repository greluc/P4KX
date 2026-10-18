use std::fmt;

use crate::{ByteOrder, Container, known};

/// The type of a chunk: a 32-bit hash ID in `#ivo`, a 16-bit ID in `CrCh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkType {
    container: Container,
    id: u32,
}

impl ChunkType {
    /// Creates the chunk type `id` of `container`.
    #[must_use]
    pub const fn new(container: Container, id: u32) -> Self {
        Self { container, id }
    }

    /// Returns the container form the ID belongs to.
    #[must_use]
    pub const fn container(self) -> Container {
        self.container
    }

    /// Returns the numeric type ID.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }

    /// Returns the name of a known type, or `None` for an unknown one.
    #[must_use]
    pub fn name(self) -> Option<&'static str> {
        known::name(self.container, self.id)
    }

    /// Returns whether the type is in the table of known chunk types.
    #[must_use]
    pub fn is_known(self) -> bool {
        self.name().is_some()
    }

    /// Returns the ID as hexadecimal text, eight digits for `#ivo` and four for `CrCh`.
    #[must_use]
    pub fn hex_id(self) -> String {
        match self.container {
            Container::Ivo => format!("0x{:08X}", self.id),
            Container::CrCh => format!("0x{:04X}", self.id),
        }
    }
}

impl fmt::Display for ChunkType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(name) => formatter.pad(name),
            None => write!(formatter, "unknown {}", self.hex_id()),
        }
    }
}

/// One entry of the chunk table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chunk {
    pub(crate) kind: ChunkType,
    pub(crate) version: u32,
    pub(crate) id: Option<u32>,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) byte_order: ByteOrder,
}

impl Chunk {
    /// Returns the chunk's type.
    #[must_use]
    pub const fn chunk_type(&self) -> ChunkType {
        self.kind
    }

    /// Returns the version of the chunk's layout, without the `CrCh` big-endian flag.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the chunk ID other chunks refer to it by; `#ivo` chunks have none.
    #[must_use]
    pub const fn id(&self) -> Option<u32> {
        self.id
    }

    /// Returns the offset of the chunk's data in the file.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the size of the chunk's data. `#ivo` stores no size: there it is the distance to the
    /// next greater offset of any chunk, the chunk table or the end of the file, alignment padding
    /// included, so chunks that share an offset share their span.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns the byte order of the chunk's data; `#ivo` chunks are always little-endian.
    #[must_use]
    pub const fn byte_order(&self) -> ByteOrder {
        self.byte_order
    }
}
