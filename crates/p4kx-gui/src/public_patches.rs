// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The public patches the RSI Launcher log named when the Channels were last detected.

use std::sync::{Arc, PoisonError, RwLock};

use p4kx_channels::PublicPatches;

static CURRENT: RwLock<Option<Arc<PublicPatches>>> = RwLock::new(None);

/// Keeps `patches` as the public patches of the jobs that start from now on.
pub(crate) fn publish(patches: PublicPatches) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(patches));
}

/// Returns the public patches published by the last detection, none before any.
pub(crate) fn current() -> Arc<PublicPatches> {
    CURRENT
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_default()
}
