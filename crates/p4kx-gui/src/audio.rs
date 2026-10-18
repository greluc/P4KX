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
    fn start(&self, source: SoundSource) -> Box<dyn Send>;
}
