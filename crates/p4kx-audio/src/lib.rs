//! Reads the Wwise sounds of Star Citizen Archives from bytes: parses WEMs (the RIFF or RIFX
//! structure and the Wwise `fmt` variants), detects the codec, writes PCM, ADPCM and Wwise Vorbis
//! sounds as WAV and rebuilds Wwise Vorbis sounds as Ogg Vorbis into a writer the caller
//! supplies; parses BNK soundbanks with their sections, their `HIRC` objects and the WEMs they
//! embed.

mod adpcm;
mod bank_error;
mod bank_parse;
mod bank_reader;
mod codec;
mod error;
mod hirc;
mod ima;
mod ogg;
mod ptadpcm;
mod riff;
mod soundbank;
mod vorbis;
mod wav;
mod wem;

pub use bank_error::{BankError, SectionId};
pub use codec::Codec;
pub use error::AudioError;
pub use hirc::{ActionTarget, HircKind, HircObject, HircValue, MediaStorage, SoundSource};
pub use ogg::OggWriter;
pub use riff::ByteOrder;
pub use soundbank::{BankHeader, EmbeddedWem, Section, Soundbank};
pub use vorbis::{Codebooks, OggReport, VorbisError};
pub use wav::WavReport;
pub use wem::Wem;
