// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{ChfError, Guid};

/// Little-endian writes of the values a CHF body holds.
pub(crate) trait Put {
    fn put_u8(&mut self, value: u8);
    fn put_u16(&mut self, value: u16);
    fn put_u32(&mut self, value: u32);
    fn put_u64(&mut self, value: u64);
    fn put_guid(&mut self, value: Guid);
}

impl Put for Vec<u8> {
    fn put_u8(&mut self, value: u8) {
        self.push(value);
    }

    fn put_u16(&mut self, value: u16) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn put_u32(&mut self, value: u32) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn put_u64(&mut self, value: u64) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn put_guid(&mut self, value: Guid) {
        self.extend_from_slice(&value.stored());
    }
}

/// Converts a length to the integer type a count field stores, or names the field that overflows.
pub(crate) fn count_as<T: TryFrom<usize>>(len: usize, what: &str) -> Result<T, ChfError> {
    T::try_from(len).map_err(|_| ChfError::Unwritable {
        reason: format!("{len} {what} do not fit their count field"),
    })
}
