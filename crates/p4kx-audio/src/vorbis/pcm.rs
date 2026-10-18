use std::fmt::Display;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};

use lewton::audio::{PreviousWindowRight, read_audio_packet_generic};
use lewton::header::{IdentHeader, SetupHeader, read_header_ident, read_header_setup};
use lewton::samples::InterleavedSamples;

use super::{Rebuild, VorbisError};
use crate::wav::wav_block_align;
use crate::{AudioError, WavReport, Wem};

const ZEROS: [u8; 4096] = [0; 4096];
const PANICKED: &str = "the decoder failed";

/// The Vorbis headers parsed by the decoder.
struct Decoder {
    ident: IdentHeader,
    setup: SetupHeader,
}

impl<B: AsRef<[u8]>> Wem<B> {
    /// Rebuilds a Wwise Vorbis sound, decodes it and writes it as a 16-bit PCM WAV to `out`.
    pub(crate) fn write_vorbis_wav(&self, out: &mut impl Write) -> Result<WavReport, AudioError> {
        let rebuild = self.rebuild_vorbis()?;
        let channels = self.channels();
        let block_align = wav_block_align(channels)?;
        let frames = rebuild.sample_count;
        let data_len = frames
            .checked_mul(u64::from(block_align))
            .and_then(|len| usize::try_from(len).ok())
            .ok_or(AudioError::WavTooLarge)?;
        let header = self.wav_header(block_align, data_len)?;
        let decoder = rebuild.decoder()?;
        out.write_all(&header).map_err(AudioError::Write)?;
        rebuild.write_pcm(&decoder, block_align, out)?;
        Ok(WavReport {
            channels,
            sample_rate: self.sample_rate(),
            sample_count: frames,
            size: (header.len() + data_len) as u64,
        })
    }
}

fn rejected(error: &dyn Display) -> AudioError {
    AudioError::InvalidVorbis(VorbisError::Rejected(error.to_string()))
}

impl Rebuild<'_> {
    fn decoder(&self) -> Result<Decoder, AudioError> {
        panic::catch_unwind(|| {
            let ident = read_header_ident(&self.ident).map_err(|error| rejected(&error))?;
            let blocksizes = (ident.blocksize_0, ident.blocksize_1);
            let setup = read_header_setup(&self.setup_packet, ident.audio_channels, blocksizes)
                .map_err(|error| rejected(&error))?;
            Ok(Decoder { ident, setup })
        })
        .unwrap_or_else(|_| Err(rejected(&PANICKED)))
    }

    /// Decodes the audio packets and writes exactly the sample count's frames of interleaved
    /// little-endian 16-bit samples, padding with silence when the decoder gives fewer.
    fn write_pcm(
        &self,
        decoder: &Decoder,
        block_align: u16,
        out: &mut impl Write,
    ) -> Result<(), AudioError> {
        let frame_len = usize::from(block_align);
        let mut audio = self.stream.audio(&self.setup);
        let mut window = PreviousWindowRight::new();
        let mut packet = Vec::new();
        let mut bytes = Vec::new();
        let mut remaining = self.sample_count;
        while remaining > 0
            && let Some(rebuilt) = audio.next_into(&mut packet)
        {
            let offset = rebuilt.map_err(AudioError::InvalidVorbis)?.offset as u64;
            let undecodable = |reason: String| {
                AudioError::InvalidVorbis(VorbisError::Undecodable { offset, reason })
            };
            let pcm: InterleavedSamples<i16> = panic::catch_unwind(AssertUnwindSafe(|| {
                read_audio_packet_generic(&decoder.ident, &decoder.setup, &packet, &mut window)
                    .map_err(|error| error.to_string())
            }))
            .unwrap_or_else(|_| Err(PANICKED.to_owned()))
            .map_err(undecodable)?;
            let available = pcm.samples.len() * 2 / frame_len;
            let take = usize::try_from(remaining).map_or(available, |left| available.min(left));
            bytes.clear();
            bytes.extend(
                pcm.samples
                    .iter()
                    .take(take * frame_len / 2)
                    .flat_map(|sample| sample.to_le_bytes()),
            );
            out.write_all(&bytes).map_err(AudioError::Write)?;
            remaining -= take as u64;
        }
        let mut padding = remaining.saturating_mul(u64::from(block_align));
        while padding > 0 {
            let len = usize::try_from(padding).map_or(ZEROS.len(), |len| len.min(ZEROS.len()));
            out.write_all(ZEROS.get(..len).unwrap_or_default())
                .map_err(AudioError::Write)?;
            padding -= len as u64;
        }
        Ok(())
    }
}
