use crate::ParseError;

/// Sequential little-endian reader over a byte slice that never reads out of bounds.
///
/// Offsets in errors are reported relative to `base`, so a reader over a slice of a larger file
/// can report file offsets.
#[derive(Debug, Clone)]
pub struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
    base: u64,
}

impl<'a> ByteReader<'a> {
    /// Creates a reader whose offsets start at zero.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self::with_base(bytes, 0)
    }

    /// Creates a reader whose reported offsets start at `base`.
    #[must_use]
    pub const fn with_base(bytes: &'a [u8], base: u64) -> Self {
        Self {
            bytes,
            position: 0,
            base,
        }
    }

    /// Returns the offset of the next byte to be read, including the base.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        self.base + self.position as u64
    }

    /// Returns the number of bytes not yet read.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    /// Reads the next `count` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::UnexpectedEnd`] when fewer than `count` bytes are left.
    pub fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], ParseError> {
        let end = self.position.checked_add(count);
        let slice = end.and_then(|end| self.bytes.get(self.position..end));
        match (slice, end) {
            (Some(slice), Some(end)) => {
                self.position = end;
                Ok(slice)
            }
            _ => Err(ParseError::UnexpectedEnd {
                offset: self.offset(),
                needed: count as u64,
                available: self.remaining() as u64,
            }),
        }
    }

    /// Skips the next `count` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::UnexpectedEnd`] when fewer than `count` bytes are left.
    pub fn skip(&mut self, count: usize) -> Result<(), ParseError> {
        self.read_bytes(count).map(|_| ())
    }

    /// Reads a little-endian `u16`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::UnexpectedEnd`] when fewer than 2 bytes are left.
    pub fn read_u16(&mut self) -> Result<u16, ParseError> {
        self.read_array().map(u16::from_le_bytes)
    }

    /// Reads a little-endian `u32`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::UnexpectedEnd`] when fewer than 4 bytes are left.
    pub fn read_u32(&mut self) -> Result<u32, ParseError> {
        self.read_array().map(u32::from_le_bytes)
    }

    /// Reads a little-endian `u64`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::UnexpectedEnd`] when fewer than 8 bytes are left.
    pub fn read_u64(&mut self) -> Result<u64, ParseError> {
        self.read_array().map(u64::from_le_bytes)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        let bytes = self.read_bytes(N)?;
        let mut array = [0; N];
        array.copy_from_slice(bytes);
        Ok(array)
    }
}
