// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cell::Cell;
use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::ToolError;

/// How many chunk files, models and textures are read at a time.
const MODEL_READERS: usize = 2;
/// How many containers are decoded at a time.
const CONTAINER_DECODERS: usize = 2;

thread_local! {
    static MODEL_HELD: Cell<bool> = const { Cell::new(false) };
}

/// Returns whether the calling thread holds a model permit.
pub(crate) fn model_held() -> bool {
    MODEL_HELD.with(Cell::get)
}

/// A number of permits for reads that may take more than 1 GiB.
///
/// A permit is taken on a blocking thread that already holds its worker permit, so the order is
/// always worker permit first, then one permit of a gate, and no call waits for a worker while
/// holding one. The model gate and the container gate are separate, so long model reads never
/// starve container decodes. A call that holds a model permit must not open a container (see
/// [`model_held`]); the other way round is not a problem, since a decode ends without waiting.
#[derive(Debug, Clone)]
pub(crate) struct Gate {
    permits: Arc<Semaphore>,
    model: bool,
}

/// A permit of a [`Gate`], released when dropped.
#[derive(Debug)]
pub(crate) struct Held {
    _permit: Option<OwnedSemaphorePermit>,
    model: bool,
}

impl Drop for Held {
    fn drop(&mut self) {
        if self.model {
            MODEL_HELD.with(|held| held.set(false));
        }
    }
}

impl Gate {
    /// Creates the permits of chunk file, model and texture reads, two at a time.
    pub(crate) fn model() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(MODEL_READERS)),
            model: true,
        }
    }

    /// Creates the permits of container decodes, two at a time.
    pub(crate) fn container() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(CONTAINER_DECODERS)),
            model: false,
        }
    }

    /// Waits for a permit on the calling blocking thread; it is held until the returned value is
    /// dropped. Outside a runtime there is nothing to wait for and no permit is held.
    pub(crate) fn hold(&self) -> Result<Held, ToolError> {
        let Ok(runtime) = Handle::try_current() else {
            return Ok(Held {
                _permit: None,
                model: false,
            });
        };
        let permit = runtime
            .block_on(Arc::clone(&self.permits).acquire_owned())
            .map_err(|_| ToolError::Stopped)?;
        if self.model {
            MODEL_HELD.with(|held| held.set(true));
        }
        Ok(Held {
            _permit: Some(permit),
            model: self.model,
        })
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::container()
    }
}
