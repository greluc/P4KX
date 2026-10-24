// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Sound playback of the audio preview: decoded PCM, the play, pause, seek and volume state the
//! window and the audio thread share, and the audio output that plays it.

mod device;
mod pcm;
mod player;
mod transport;

pub use device::DeviceOutput;
pub use pcm::{MAX_PREVIEW_PCM, Pcm};
pub(crate) use pcm::{Undecoded, decode};
pub use player::Player;
pub use transport::{FailureReporter, SoundSource};

/// Plays sounds; the window holds one and starts a stream on it for every sound played.
pub trait AudioOutput {
    /// Starts pulling samples from `source` without blocking the caller and returns a guard
    /// that ends the stream when dropped. A failure, at the start or later, goes to the window
    /// through [`SoundSource::reporter`].
    fn start(&self, source: SoundSource) -> StreamGuard;
}

/// Ends an output stream when dropped, by dropping what the output keeps the stream alive with.
pub struct StreamGuard {
    _keep: Box<dyn Send>,
}

impl StreamGuard {
    /// Wraps `keep`, whose drop ends the stream.
    #[must_use]
    pub fn new(keep: impl Send + 'static) -> Self {
        Self {
            _keep: Box::new(keep),
        }
    }
}

impl std::fmt::Debug for StreamGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamGuard").finish_non_exhaustive()
    }
}
