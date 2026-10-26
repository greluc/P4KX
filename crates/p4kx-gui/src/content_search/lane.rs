// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::thread;
use std::time::Duration;

const POLL: Duration = Duration::from_millis(5);

/// The one place a search of the tab runs in: a search entering it waits until the one before it
/// has ended, so that a cancelled search still winding down never runs beside the next one and
/// the two never hold their memory together.
#[derive(Debug, Default)]
pub(crate) struct Lane {
    turn: Mutex<()>,
}

impl Lane {
    /// Waits for the search before to end and returns the turn, or `None` once `cancelled` is set
    /// while waiting.
    pub(crate) fn enter(&self, cancelled: &AtomicBool) -> Option<MutexGuard<'_, ()>> {
        loop {
            match self.turn.try_lock() {
                Ok(turn) => return Some(turn),
                Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
                Err(TryLockError::WouldBlock) => {
                    if cancelled.load(Ordering::Relaxed) {
                        return None;
                    }
                    thread::sleep(POLL);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use super::Lane;

    #[test]
    fn a_search_waits_until_the_one_before_it_has_left() {
        let lane = Arc::new(Lane::default());
        let first = lane.enter(&AtomicBool::new(false));
        let waiting = {
            let lane = Arc::clone(&lane);
            thread::spawn(move || lane.enter(&AtomicBool::new(false)).is_some())
        };

        thread::sleep(Duration::from_millis(50));
        assert!(!waiting.is_finished(), "still waiting for the first");
        drop(first);

        assert_eq!(
            waiting.join().ok(),
            Some(true),
            "entered after the first left"
        );
    }

    #[test]
    fn a_cancelled_search_stops_waiting() {
        let lane = Arc::new(Lane::default());
        let _first = lane.enter(&AtomicBool::new(false));
        let cancelled = Arc::new(AtomicBool::new(false));
        let waiting = {
            let (lane, cancelled) = (Arc::clone(&lane), Arc::clone(&cancelled));
            thread::spawn(move || lane.enter(&cancelled).is_none())
        };

        cancelled.store(true, Ordering::Relaxed);

        assert_eq!(waiting.join().ok(), Some(true), "gave up waiting");
    }
}
