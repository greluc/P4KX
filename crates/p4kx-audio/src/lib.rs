//! Reads the Wwise sounds of Star Citizen Archives from bytes: parses WEMs (the RIFF or RIFX
//! structure and the Wwise `fmt` variants), detects the codec, writes PCM, ADPCM, Wwise Vorbis
//! and Opus sounds as WAV, rebuilds Wwise Vorbis sounds as Ogg Vorbis and repackages Opus sounds
//! as Ogg Opus into a writer the caller supplies; parses BNK soundbanks with their sections, their
//! `HIRC` objects and the WEMs they embed.

mod adpcm;
mod bank_error;
mod bank_parse;
mod bank_reader;
mod codec;
mod error;
mod hirc;
mod ima;
mod loops;
mod ogg;
mod opus;
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
pub use loops::Loop;
pub use ogg::OggWriter;
pub use opus::OpusError;
pub use riff::ByteOrder;
pub use soundbank::{BankHeader, EmbeddedWem, Section, Soundbank};
pub use vorbis::{Codebooks, OggReport, VorbisError};
pub use wav::WavReport;
pub use wem::Wem;
