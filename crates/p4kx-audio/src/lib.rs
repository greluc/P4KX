//! Reads the Wwise WEM sounds of Star Citizen Archives from bytes: parses the RIFF or RIFX
//! structure and the Wwise `fmt` variants, detects the codec, writes PCM sounds as WAV and
//! rebuilds Wwise Vorbis sounds as Ogg Vorbis into a writer the caller supplies.

mod codec;
mod error;
mod ogg;
mod riff;
mod vorbis;
mod wav;
mod wem;

pub use codec::Codec;
pub use error::AudioError;
pub use ogg::OggWriter;
pub use riff::ByteOrder;
pub use vorbis::{Codebooks, OggReport, VorbisError};
pub use wav::WavReport;
pub use wem::Wem;
