// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// How far building an Interior has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InteriorProgress {
    /// The container at position `done` of the `total` an Entity names is about to be built.
    Container {
        /// How many containers are done.
        done: usize,
        /// How many containers the Entity names.
        total: usize,
    },
    /// The models of the container being built are being read.
    Models {
        /// How many distinct model files are read.
        done: usize,
        /// How many distinct model files the container places.
        total: usize,
    },
}

/// The ways to stop building an Interior and to watch it.
#[derive(Clone, Copy)]
pub struct InteriorWatch<'a> {
    /// Returns whether building should stop; it then ends with
    /// [`AssemblyError::Cancelled`](crate::AssemblyError::Cancelled).
    pub cancelled: &'a (dyn Fn() -> bool + Sync),
    /// Called as building advances, from any thread.
    pub progress: &'a (dyn Fn(InteriorProgress) + Sync),
}

impl InteriorWatch<'_> {
    /// Returns a watch that never stops building and ignores its progress.
    #[must_use]
    pub const fn none() -> InteriorWatch<'static> {
        InteriorWatch {
            cancelled: &|| false,
            progress: &|_| {},
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        (self.cancelled)()
    }

    pub(crate) fn report(&self, progress: InteriorProgress) {
        (self.progress)(progress);
    }
}

impl std::fmt::Debug for InteriorWatch<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InteriorWatch").finish_non_exhaustive()
    }
}
