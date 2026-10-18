use std::marker::PhantomData;

use p4kx_foundation::ByteReader;

use crate::DataForgeError;

/// A fixed-size row of a DataForge table, decoded field by field in little-endian order.
pub(crate) trait Row: Copy {
    /// The number of bytes of one row.
    const SIZE: usize;

    /// Decodes one row of exactly [`Row::SIZE`] bytes.
    fn decode(fields: &mut Fields<'_>) -> Self;
}

/// Little-endian fields of one table row; a read past the row yields zeros.
pub(crate) struct Fields<'a>(&'a [u8]);

impl<'a> Fields<'a> {
    pub(crate) const fn new(row: &'a [u8]) -> Self {
        Self(row)
    }
}

impl Fields<'_> {
    const fn take<const N: usize>(&mut self) -> [u8; N] {
        match self.0.split_first_chunk::<N>() {
            Some((head, rest)) => {
                self.0 = rest;
                *head
            }
            None => [0; N],
        }
    }

    pub(crate) const fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.take())
    }

    pub(crate) const fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }

    pub(crate) const fn bytes16(&mut self) -> [u8; 16] {
        self.take()
    }
}

/// A typed view of a table of rows inside the DataForge bytes.
#[derive(Debug)]
pub(crate) struct Table<'a, T> {
    bytes: &'a [u8],
    row: PhantomData<fn() -> T>,
}

impl<T> Clone for Table<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Table<'_, T> {}

impl<'a, T: Row> Table<'a, T> {
    /// Takes a table of `count` rows from `reader`.
    pub(crate) fn read(
        reader: &mut ByteReader<'a>,
        count: u32,
        part: &'static str,
    ) -> Result<Self, DataForgeError> {
        Ok(Self::from_bytes(take(reader, count, T::SIZE, part)?))
    }

    /// Views `bytes` as rows; a partial row at the end is ignored.
    pub(crate) const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            row: PhantomData,
        }
    }

    pub(crate) const fn len(&self) -> usize {
        self.bytes.len() / T::SIZE
    }

    pub(crate) fn get(&self, index: usize) -> Option<T> {
        let start = index.checked_mul(T::SIZE)?;
        let row = self.bytes.get(start..start.checked_add(T::SIZE)?)?;
        Some(T::decode(&mut Fields(row)))
    }

    pub(crate) fn iter_indexed(&self) -> impl ExactSizeIterator<Item = (u32, T)> + use<'a, T> {
        let len = u32::try_from(self.len()).unwrap_or(u32::MAX);
        (0..len).zip(self.iter())
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = T> + use<'a, T> {
        self.bytes
            .chunks_exact(T::SIZE)
            .map(|row| T::decode(&mut Fields(row)))
    }
}

/// Takes `count` elements of `size` bytes each from `reader`.
pub(crate) fn take<'a>(
    reader: &mut ByteReader<'a>,
    count: u32,
    size: usize,
    part: &'static str,
) -> Result<&'a [u8], DataForgeError> {
    let len = usize::try_from(count)
        .ok()
        .and_then(|count| count.checked_mul(size))
        .ok_or(DataForgeError::TooLarge { part })?;
    reader
        .read_bytes(len)
        .map_err(|source| DataForgeError::Truncated { part, source })
}
