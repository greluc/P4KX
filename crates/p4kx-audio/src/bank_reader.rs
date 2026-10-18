use crate::{BankError, SectionId};

const MAX_VARIABLE_LENGTH_BYTES: usize = 5;

/// Reads little-endian fields from the body of one section, failing with a [`BankError`] that
/// names the section.
#[derive(Debug, Clone)]
pub(crate) struct SectionReader<'a> {
    section: SectionId,
    body: &'a [u8],
    position: usize,
}

impl<'a> SectionReader<'a> {
    pub(crate) const fn new(section: SectionId, body: &'a [u8]) -> Self {
        Self {
            section,
            body,
            position: 0,
        }
    }

    pub(crate) const fn remaining(&self) -> usize {
        self.body.len().saturating_sub(self.position)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, BankError> {
        let [byte] = self.array::<1>()?;
        Ok(byte)
    }

    pub(crate) fn u16(&mut self) -> Result<u16, BankError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, BankError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// Reads a Wwise variable-length integer: 7 bits per byte, most significant first, the high
    /// bit set on every byte but the last.
    pub(crate) fn variable_length(&mut self) -> Result<u32, BankError> {
        let mut value: u32 = 0;
        for _ in 0..MAX_VARIABLE_LENGTH_BYTES {
            let byte = self.u8()?;
            value = value.checked_mul(0x80).ok_or_else(|| {
                BankError::invalid(self.section, "a variable-length integer overflows")
            })? | u32::from(byte & 0x7F);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(BankError::invalid(
            self.section,
            "a variable-length integer is longer than 5 bytes",
        ))
    }

    /// Skips `len` bytes and returns their range in the body.
    pub(crate) fn skip(&mut self, len: usize) -> Result<std::ops::Range<usize>, BankError> {
        let start = self.position;
        let end = start
            .checked_add(len)
            .filter(|&end| end <= self.body.len())
            .ok_or_else(|| self.truncated(len))?;
        self.position = end;
        Ok(start..end)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], BankError> {
        let field = self
            .body
            .get(self.position..)
            .and_then(<[u8]>::first_chunk::<N>)
            .copied()
            .ok_or_else(|| self.truncated(N))?;
        self.position += N;
        Ok(field)
    }

    const fn truncated(&self, len: usize) -> BankError {
        BankError::truncated(
            self.section,
            self.body.len(),
            self.position as u64 + len as u64,
        )
    }
}
