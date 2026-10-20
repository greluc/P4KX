use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, RwLock};

use p4kx_foundation::ArchiveKey;

/// The outcome of checking a key: it passed, or it failed on the named encrypted Entry.
pub(crate) type Outcome = Result<(), String>;

#[derive(Default)]
struct State {
    key: Option<ArchiveKey>,
    outcome: Option<Outcome>,
    wanted: AtomicBool,
}

/// The Archive key shared by an Archive and the containers opened from it, with the outcome of
/// checking it, so that a wrong key is found once and not again for every Entry.
#[derive(Clone, Default)]
pub(crate) struct KeySlot(Arc<RwLock<State>>);

impl KeySlot {
    pub(crate) fn with(key: Option<ArchiveKey>) -> Self {
        let slot = Self::default();
        if let Some(key) = key {
            slot.set(key);
        }
        slot
    }

    pub(crate) fn set(&self, key: ArchiveKey) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = State {
            key: Some(key),
            ..State::default()
        };
    }

    /// Notes that an encrypted Entry was needed while no key fit.
    pub(crate) fn want(&self) {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .wanted
            .store(true, Ordering::Relaxed);
    }

    /// Returns whether a key was wanted since the last call.
    pub(crate) fn take_wanted(&self) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .wanted
            .swap(false, Ordering::Relaxed)
    }

    pub(crate) fn clear(&self) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = State::default();
    }

    pub(crate) fn is_set(&self) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .key
            .is_some()
    }

    /// Returns the key and the outcome of checking it, if it has been checked.
    pub(crate) fn get(&self) -> Option<(ArchiveKey, Option<Outcome>)> {
        let state = self.0.read().unwrap_or_else(PoisonError::into_inner);
        state.key.clone().map(|key| (key, state.outcome.clone()))
    }

    /// Records the outcome of checking `key`, unless another key was set meanwhile.
    pub(crate) fn record(&self, key: &ArchiveKey, outcome: Outcome) {
        let mut state = self.0.write().unwrap_or_else(PoisonError::into_inner);
        if state.key.as_ref() == Some(key) {
            if outcome.is_err() {
                state.wanted.store(true, Ordering::Relaxed);
            }
            state.outcome = Some(outcome);
        }
    }
}

impl fmt::Debug for KeySlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeySlot")
            .field("set", &self.is_set())
            .finish()
    }
}
