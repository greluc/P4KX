//! Reads the Wwise WEM sounds of Star Citizen Archives from bytes: parses the RIFF or RIFX
//! structure and the Wwise `fmt` variants, detects the codec and writes PCM sounds as WAV into a
//! writer the caller supplies.

mod codec;
mod error;
mod riff;
mod wav;
mod wem;

pub use codec::Codec;
pub use error::AudioError;
pub use riff::ByteOrder;
pub use wav::WavReport;
pub use wem::Wem;
