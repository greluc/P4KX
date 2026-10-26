// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Mutex, PoisonError};

use p4kx_extraction::Soundbank;

type Kept = Option<(u32, Arc<Soundbank<Vec<u8>>>)>;

/// The soundbank read last for playing one of its embedded WEMs, kept so that the next WEM of the
/// same soundbank is not read again. Clones share the one slot.
#[derive(Debug, Clone, Default)]
pub(crate) struct BankCache(Arc<Mutex<Kept>>);

impl BankCache {
    /// Returns the kept soundbank when it is the one of Entry `id`.
    pub(crate) fn get(&self, id: u32) -> Option<Arc<Soundbank<Vec<u8>>>> {
        let kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        kept.as_ref()
            .filter(|(kept, _)| *kept == id)
            .map(|(_, bank)| Arc::clone(bank))
    }

    /// Keeps `bank`, the soundbank of Entry `id`, instead of the one kept before.
    pub(crate) fn keep(&self, id: u32, bank: &Arc<Soundbank<Vec<u8>>>) {
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *kept = Some((id, Arc::clone(bank)));
    }

    /// Returns the Entry of the kept soundbank.
    pub(crate) fn kept(&self) -> Option<u32> {
        let kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        kept.as_ref().map(|(id, _)| *id)
    }

    /// Drops the kept soundbank.
    pub(crate) fn clear(&self) {
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *kept = None;
    }
}
