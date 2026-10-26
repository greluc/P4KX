// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cell::Cell;

thread_local! {
    static SHAPED: Cell<usize> = const { Cell::new(0) };
    static REUSE: Cell<bool> = const { Cell::new(true) };
}

/// Counts one shaping of a text on this thread.
pub(super) fn shaped() {
    SHAPED.with(|count| count.set(count.get() + 1));
}

/// Returns how many texts this thread shaped since the last call, and starts again from zero.
pub(crate) fn take_shaped() -> usize {
    SHAPED.with(|count| count.replace(0))
}

/// Returns whether the paint pass may reuse a text the measuring pass shaped.
pub(super) fn reuse() -> bool {
    REUSE.with(Cell::get)
}

/// Allows or forbids the paint pass to reuse the texts the measuring pass shaped.
pub(crate) fn set_reuse(allowed: bool) {
    REUSE.with(|reuse| reuse.set(allowed));
}
