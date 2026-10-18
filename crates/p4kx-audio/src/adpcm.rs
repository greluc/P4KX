//! Streams the blocks of an IMA ADPCM or PTADPCM WEM through their decoders into 16-bit PCM.

use std::io::Write;

use rayon::prelude::*;

use crate::{AudioError, ByteOrder, Codec, ima, ptadpcm};

const BATCH_BYTES: usize = 1 << 20;
const MIN_BLOCKS_PER_TASK: usize = 16;

/// The ADPCM codecs with a decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ima,
    PtAdpcm,
}

impl Kind {
    const fn header_len(self) -> usize {
        match self {
            Self::Ima => ima::HEADER_LEN,
            Self::PtAdpcm => ptadpcm::HEADER_LEN,
        }
    }

    const fn samples(self, sub_block: usize) -> usize {
        match self {
            Self::Ima => ima::samples_per_sub_block(sub_block),
            Self::PtAdpcm => ptadpcm::samples_per_frame(sub_block),
        }
    }
}

/// The block layout of an ADPCM WEM: every block holds one contiguous sub-block per channel.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    kind: Kind,
    channels: usize,
    sub_block: usize,
}

impl Layout {
    /// Returns the layout of an ADPCM codec, or `None` when the codec is not ADPCM or the block
    /// alignment does not split into sub-blocks with at least one sample after the header.
    pub(crate) fn of(codec: Codec, channels: u16, block_align: u16) -> Option<Self> {
        let kind = match codec {
            Codec::ImaAdpcm => Kind::Ima,
            Codec::PtAdpcm => Kind::PtAdpcm,
            _ => return None,
        };
        let channels = usize::from(channels);
        let block_align = usize::from(block_align);
        let sub_block = block_align.checked_div(channels)?;
        (block_align % channels == 0 && sub_block > kind.header_len()).then_some(Self {
            kind,
            channels,
            sub_block,
        })
    }

    /// Returns the frames, samples per channel, that `data_len` bytes decode to.
    pub(crate) fn frames(self, data_len: u64) -> u64 {
        let block = (self.sub_block * self.channels) as u64;
        let rest = usize::try_from(data_len % block).unwrap_or(0);
        let partial = self
            .partial_sub_block(rest)
            .map_or(0, |len| self.kind.samples(len));
        data_len / block * self.kind.samples(self.sub_block) as u64 + partial as u64
    }

    /// Decodes `data` and writes it to `out` as interleaved little-endian 16-bit samples.
    pub(crate) fn write_pcm(
        self,
        data: &[u8],
        order: ByteOrder,
        out: &mut impl Write,
    ) -> Result<(), AudioError> {
        let block = self.sub_block * self.channels;
        let block_pcm = self.kind.samples(self.sub_block) * self.channels * 2;
        let (blocks, rest) = data
            .split_at_checked(data.len() - data.len() % block)
            .unwrap_or((data, &[]));
        let batch_blocks = (BATCH_BYTES / block_pcm).max(1);
        let mut buffer = vec![0; block_pcm * batch_blocks.min(blocks.len() / block)];
        for batch in blocks.chunks(block * batch_blocks) {
            let pcm = buffer
                .get_mut(..batch.len() / block * block_pcm)
                .unwrap_or_default();
            pcm.par_chunks_mut(block_pcm)
                .zip(batch.par_chunks(block))
                .with_min_len(MIN_BLOCKS_PER_TASK)
                .for_each(|(pcm, block)| self.decode_block(block, self.sub_block, order, pcm));
            out.write_all(pcm).map_err(AudioError::Write)?;
        }
        if let Some(sub_block) = self.partial_sub_block(rest.len()) {
            let mut pcm = vec![0; self.kind.samples(sub_block) * self.channels * 2];
            self.decode_block(rest, sub_block, order, &mut pcm);
            out.write_all(&pcm).map_err(AudioError::Write)?;
        }
        Ok(())
    }

    fn decode_block(self, block: &[u8], sub_block: usize, order: ByteOrder, pcm: &mut [u8]) {
        let samples = self.kind.samples(sub_block);
        let (slots, _) = pcm.as_chunks_mut::<2>();
        for (channel, sub) in block
            .chunks_exact(sub_block)
            .take(self.channels)
            .enumerate()
        {
            let out = slots
                .iter_mut()
                .skip(channel)
                .step_by(self.channels)
                .take(samples);
            match self.kind {
                Kind::Ima => ima::decode(sub, order, out),
                Kind::PtAdpcm => ptadpcm::decode(sub, out),
            }
        }
    }

    fn partial_sub_block(self, rest: usize) -> Option<usize> {
        let sub_block = rest / self.channels;
        (self.kind == Kind::Ima && sub_block > ima::HEADER_LEN).then_some(sub_block)
    }
}

/// Returns the nibbles after a header of `header_len` bytes, the low nibble of each byte first.
pub(crate) fn nibbles(sub_block: &[u8], header_len: usize) -> impl Iterator<Item = u8> {
    sub_block
        .get(header_len..)
        .unwrap_or_default()
        .iter()
        .flat_map(|&byte| [byte & 0x0F, byte >> 4])
}

/// Saturates `sample` to 16 bits.
pub(crate) fn clamp16(sample: i32) -> i32 {
    sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX))
}

/// Returns a sample already within 16 bits as little-endian bytes.
pub(crate) fn sample_bytes(sample: i32) -> [u8; 2] {
    i16::try_from(clamp16(sample))
        .unwrap_or_default()
        .to_le_bytes()
}
