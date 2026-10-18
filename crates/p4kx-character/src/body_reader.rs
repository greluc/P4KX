use p4kx_foundation::ByteReader;

use crate::{ChfError, Guid};

/// A bounds-checked reader over a decompressed CHF body with the checks its layout needs.
pub(crate) struct BodyReader<'a> {
    inner: ByteReader<'a>,
}

impl<'a> BodyReader<'a> {
    pub(crate) const fn new(body: &'a [u8]) -> Self {
        Self {
            inner: ByteReader::new(body),
        }
    }

    pub(crate) const fn with_base(bytes: &'a [u8], base: u64) -> Self {
        Self {
            inner: ByteReader::with_base(bytes, base),
        }
    }

    pub(crate) const fn offset(&self) -> u64 {
        self.inner.offset()
    }

    pub(crate) const fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    pub(crate) fn u8(&mut self) -> Result<u8, ChfError> {
        let bytes = self.inner.read_bytes(1)?;
        Ok(bytes.first().copied().unwrap_or_default())
    }

    pub(crate) fn u16(&mut self) -> Result<u16, ChfError> {
        Ok(self.inner.read_u16()?)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, ChfError> {
        Ok(self.inner.read_u32()?)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, ChfError> {
        Ok(self.inner.read_u64()?)
    }

    pub(crate) fn guid(&mut self) -> Result<Guid, ChfError> {
        let mut stored = [0; 16];
        stored.copy_from_slice(self.inner.read_bytes(16)?);
        Ok(Guid::from_stored(stored))
    }

    pub(crate) fn bytes(&mut self, count: usize) -> Result<&'a [u8], ChfError> {
        Ok(self.inner.read_bytes(count)?)
    }

    /// Reads a `u32` that the layout fixes to `expected`.
    pub(crate) fn expect_u32(&mut self, what: &'static str, expected: u32) -> Result<(), ChfError> {
        let offset = self.offset();
        let found = self.u32()?;
        if found == expected {
            Ok(())
        } else {
            Err(unexpected(offset, what, expected.into(), found.into()))
        }
    }

    /// Reads a GUID that the layout fixes to all zeros.
    pub(crate) fn expect_empty_guid(&mut self, what: &'static str) -> Result<(), ChfError> {
        let offset = self.offset();
        let found = u128::from_le_bytes(self.guid()?.stored());
        if found == 0 {
            Ok(())
        } else {
            Err(unexpected(offset, what, 0, found))
        }
    }

    /// Checks that at least `len` bytes are left.
    pub(crate) fn ensure_remaining(&self, len: usize) -> Result<(), ChfError> {
        self.count(1, len).map(|_| ())
    }

    /// Checks that `count` items of at least `item_len` bytes each can still follow, so a count
    /// from the body never sizes an allocation larger than the body itself.
    pub(crate) fn count(&self, count: u64, item_len: usize) -> Result<usize, ChfError> {
        let needed = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(item_len));
        match (usize::try_from(count), needed) {
            (Ok(count), Some(needed)) if needed <= self.remaining() => Ok(count),
            _ => Err(p4kx_foundation::ParseError::UnexpectedEnd {
                offset: self.offset(),
                needed: count.saturating_mul(item_len as u64),
                available: self.remaining() as u64,
            }
            .into()),
        }
    }
}

/// An [`ChfError::UnexpectedValue`] for a field at `offset`.
pub(crate) const fn unexpected(
    offset: u64,
    what: &'static str,
    expected: u128,
    found: u128,
) -> ChfError {
    ChfError::UnexpectedValue {
        offset,
        what,
        expected,
        found,
    }
}
