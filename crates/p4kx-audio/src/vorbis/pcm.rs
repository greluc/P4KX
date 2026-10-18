use std::fmt::Display;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};

use lewton::audio::{PreviousWindowRight, read_audio_packet_generic};
use lewton::header::{IdentHeader, SetupHeader, read_header_ident, read_header_setup};
use lewton::samples::InterleavedSamples;

use super::{RebuiltSound, VorbisError};
use crate::{AudioError, WavReport, Wem};

const PANICKED: &str = "the decoder failed";

/// The Vorbis headers parsed by the decoder.
struct Decoder {
    ident: IdentHeader,
    setup: SetupHeader,
}

impl<B: AsRef<[u8]>> Wem<B> {
    /// Rebuilds a Wwise Vorbis sound, decodes it and writes it as a 16-bit PCM WAV to `out`.
    pub(crate) fn write_vorbis_wav(&self, out: &mut impl Write) -> Result<WavReport, AudioError> {
        let sound = self.rebuild_vorbis()?;
        let (header, report) = self.decoded_wav_header(sound.sample_count)?;
        let decoder = sound.decoder()?;
        out.write_all(&header).map_err(AudioError::Write)?;
        sound.write_pcm(&decoder, usize::from(self.channels()).max(1), out)?;
        Ok(report)
    }
}

fn rejected(error: &dyn Display) -> AudioError {
    AudioError::InvalidVorbis(VorbisError::Rejected(error.to_string()))
}

impl RebuiltSound<'_> {
    fn decoder(&self) -> Result<Decoder, AudioError> {
        panic::catch_unwind(|| {
            let ident = read_header_ident(&self.ident).map_err(|error| rejected(&error))?;
            let blocksizes = (ident.blocksize_0, ident.blocksize_1);
            let setup = read_header_setup(&self.setup_header, ident.audio_channels, blocksizes)
                .map_err(|error| rejected(&error))?;
            Ok(Decoder { ident, setup })
        })
        .unwrap_or_else(|_| Err(rejected(&PANICKED)))
    }

    /// Decodes the audio packets and writes the sample count's frames of interleaved
    /// little-endian 16-bit samples.
    fn write_pcm(
        &self,
        decoder: &Decoder,
        channels: usize,
        out: &mut impl Write,
    ) -> Result<(), AudioError> {
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
            let available = pcm.samples.len().checked_div(channels).unwrap_or_default();
            let take = usize::try_from(remaining).map_or(available, |left| available.min(left));
            bytes.clear();
            for frame in pcm.samples.chunks_exact(channels).take(take) {
                for sample in frame {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
            }
            out.write_all(&bytes).map_err(AudioError::Write)?;
            remaining -= take as u64;
        }
        if remaining > 0 {
            return Err(AudioError::InvalidVorbis(VorbisError::MissingSamples {
                expected: self.sample_count,
                decoded: self.sample_count - remaining,
            }));
        }
        Ok(())
    }
}
