use std::io::Write;

use crate::{AudioError, ByteOrder, Codec, Wem};

const BITS_PER_SAMPLE: u16 = 16;
const FORMAT_PCM: u16 = 0x0001;
const FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const PCM_FMT_LEN: u32 = 16;
const EXTENSIBLE_FMT_LEN: u32 = 40;
const EXTENSION_LEN: u16 = 22;
const SUBTYPE_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
const SWAP_BUFFER_LEN: usize = 64 * 1024;

/// What [`Wem::write_wav`] wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavReport {
    /// Channels of the WAV.
    pub channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Frames written, samples per channel.
    pub sample_count: u64,
    /// Size of the whole WAV file in bytes.
    pub size: u64,
}

impl<B: AsRef<[u8]>> Wem<B> {
    /// Writes the sound as a 16-bit PCM WAV file to `out`: a canonical `WAVE_FORMAT_PCM` header
    /// for one or two channels, `WAVE_FORMAT_EXTENSIBLE` with the channel mask for more. A
    /// trailing partial frame is dropped.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::UnsupportedCodec`] before writing anything when the codec is not
    /// PCM, [`AudioError::WavTooLarge`] when the WAV would exceed 4 GiB, and
    /// [`AudioError::Write`] when `out` fails.
    pub fn write_wav(&self, out: &mut impl Write) -> Result<WavReport, AudioError> {
        if self.codec() != Codec::Pcm {
            return Err(AudioError::UnsupportedCodec(self.codec()));
        }
        let channels = self.channels();
        let block_align = channels
            .checked_mul(BITS_PER_SAMPLE / 8)
            .ok_or(AudioError::InvalidFormat("too many channels"))?;
        let frames = self.data().len() / usize::from(block_align);
        let data = self
            .data()
            .get(..frames * usize::from(block_align))
            .unwrap_or_default();
        let header = self.wav_header(block_align, data.len())?;
        out.write_all(&header).map_err(AudioError::Write)?;
        match self.byte_order() {
            ByteOrder::Little => out.write_all(data).map_err(AudioError::Write)?,
            ByteOrder::Big => write_swapped(data, out)?,
        }
        Ok(WavReport {
            channels,
            sample_rate: self.sample_rate(),
            sample_count: frames as u64,
            size: (header.len() + data.len()) as u64,
        })
    }

    fn wav_header(&self, block_align: u16, data_len: usize) -> Result<Vec<u8>, AudioError> {
        let channels = self.channels();
        let extensible = channels > 2;
        let fmt_len = if extensible {
            EXTENSIBLE_FMT_LEN
        } else {
            PCM_FMT_LEN
        };
        let data_len = u32::try_from(data_len).map_err(|_| AudioError::WavTooLarge)?;
        let riff_len = (4 + 8 + 8 + fmt_len)
            .checked_add(data_len)
            .ok_or(AudioError::WavTooLarge)?;
        let byte_rate = self
            .sample_rate()
            .checked_mul(u32::from(block_align))
            .ok_or(AudioError::InvalidFormat("the byte rate exceeds 32 bits"))?;
        let mut header = Vec::with_capacity(68);
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&riff_len.to_le_bytes());
        header.extend_from_slice(b"WAVEfmt ");
        header.extend_from_slice(&fmt_len.to_le_bytes());
        let tag = if extensible {
            FORMAT_EXTENSIBLE
        } else {
            FORMAT_PCM
        };
        header.extend_from_slice(&tag.to_le_bytes());
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&self.sample_rate().to_le_bytes());
        header.extend_from_slice(&byte_rate.to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
        if extensible {
            let mask = self.channel_mask();
            let mask = if mask.count_ones() == u32::from(channels) {
                mask
            } else {
                0
            };
            header.extend_from_slice(&EXTENSION_LEN.to_le_bytes());
            header.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
            header.extend_from_slice(&mask.to_le_bytes());
            header.extend_from_slice(&SUBTYPE_PCM);
        }
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_len.to_le_bytes());
        Ok(header)
    }
}

fn write_swapped(data: &[u8], out: &mut impl Write) -> Result<(), AudioError> {
    let mut buffer = vec![0; SWAP_BUFFER_LEN.min(data.len())];
    for chunk in data.chunks(SWAP_BUFFER_LEN) {
        let (samples, _) = chunk.as_chunks::<2>();
        let (swapped, _) = buffer.as_chunks_mut::<2>();
        for (target, sample) in swapped.iter_mut().zip(samples) {
            let [high, low] = *sample;
            *target = [low, high];
        }
        out.write_all(buffer.get(..chunk.len()).unwrap_or_default())
            .map_err(AudioError::Write)?;
    }
    Ok(())
}
