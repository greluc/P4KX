// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A cache that keeps one resource, replacing it when another key is asked for. The previous
/// resource is dropped at once, so switching models never holds two.
#[derive(Debug)]
pub(super) struct Slot<T> {
    held: Option<(u64, T)>,
    stored: u64,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            held: None,
            stored: 0,
        }
    }
}

impl<T> Slot<T> {
    pub(super) fn get(&self, key: u64) -> Option<&T> {
        self.held
            .as_ref()
            .filter(|(held, _)| *held == key)
            .map(|(_, value)| value)
    }

    pub(super) fn key(&self) -> Option<u64> {
        self.held.as_ref().map(|(key, _)| *key)
    }

    /// Keeps `value` under `key` and drops what was held.
    pub(super) fn store(&mut self, key: u64, value: T) {
        self.held = Some((key, value));
        self.stored += 1;
    }

    pub(super) fn clear(&mut self) {
        self.held = None;
    }

    /// Returns how often a resource was stored, to tell a re-upload from a cache hit.
    #[cfg(test)]
    pub(super) const fn stored(&self) -> u64 {
        self.stored
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::Slot;

    struct Counted(Rc<Cell<u32>>);

    impl Drop for Counted {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn storing_another_key_drops_the_previous_resource_at_once() {
        let drops = Rc::new(Cell::new(0));
        let mut slot = Slot::default();

        slot.store(1, Counted(Rc::clone(&drops)));
        assert_eq!(drops.get(), 0, "held");
        slot.store(2, Counted(Rc::clone(&drops)));

        assert_eq!(drops.get(), 1, "the first was dropped");
        assert!(slot.get(1).is_none(), "gone");
        assert!(slot.get(2).is_some(), "the second is held");
        assert_eq!(slot.key(), Some(2), "key");
    }

    #[test]
    fn many_switches_hold_exactly_one_resource() {
        let drops = Rc::new(Cell::new(0));
        let mut slot = Slot::default();

        for key in 0..100 {
            slot.store(key, Counted(Rc::clone(&drops)));
        }

        assert_eq!(drops.get(), 99, "every earlier resource was dropped");
        assert_eq!(slot.stored(), 100, "stores counted");
        slot.clear();
        assert_eq!(drops.get(), 100, "clearing drops the last");
    }
}
