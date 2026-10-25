// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::ToolError;

/// How many chunk files, models, textures and containers are decoded at a time.
const READERS: usize = 2;

/// The permits for the reads that may take more than 1 GiB: chunk files, models, textures and the
/// decode of a container.
///
/// A permit is taken on a blocking thread that already holds its worker permit, so the order is
/// always worker permit first, then this one, and no call waits for a worker while holding it.
#[derive(Debug, Clone)]
pub(crate) struct ModelGate {
    permits: Arc<Semaphore>,
}

impl Default for ModelGate {
    fn default() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(READERS)),
        }
    }
}

impl ModelGate {
    /// Waits for a permit on the calling blocking thread; it is held until the returned value is
    /// dropped. Outside a runtime there is nothing to wait for and no permit is held.
    pub(crate) fn hold(&self) -> Result<Option<OwnedSemaphorePermit>, ToolError> {
        let Ok(runtime) = Handle::try_current() else {
            return Ok(None);
        };
        runtime
            .block_on(Arc::clone(&self.permits).acquire_owned())
            .map(Some)
            .map_err(|_| ToolError::Stopped)
    }
}
