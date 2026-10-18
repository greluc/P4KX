//! Streams the blocks of an IMA ADPCM or PTADPCM WEM through their decoders into 16-bit PCM.

use std::io::Write;

use rayon::prelude::*;

use crate::{AudioError, ByteOrder, Codec, ima, ptadpcm};

const BATCH_BYTES: usize = 1 << 20;
const MIN_BLOCKS_PER_TASK: usize = 16;

/// The block layout of an ADPCM WEM: every block holds one contiguous sub-block per channel.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    codec: Codec,
    channels: usize,
    sub_block: usize,
}

impl Layout {
    /// Returns the layout of an ADPCM codec, or `None` when the codec is not ADPCM or the block
    /// alignment does not split into sub-blocks with at least one sample after the header.
    pub(crate) fn of(codec: Codec, channels: u16, block_align: u16) -> Option<Self> {
        let channels = usize::from(channels);
        let block_align = usize::from(block_align);
        let header = header_len(codec)?;
        let sub_block = block_align.checked_div(channels)?;
        (block_align % channels == 0 && sub_block > header).then_some(Self {
            codec,
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
            .map_or(0, |len| self.samples(len));
        data_len / block * self.samples(self.sub_block) as u64 + partial as u64
    }

    /// Decodes `data` and writes it to `out` as interleaved little-endian 16-bit samples.
    pub(crate) fn write_pcm(
        self,
        data: &[u8],
        order: ByteOrder,
        out: &mut impl Write,
    ) -> Result<(), AudioError> {
        let block = self.sub_block * self.channels;
        let block_pcm = self.samples(self.sub_block) * self.channels * 2;
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
            let mut pcm = vec![0; self.samples(sub_block) * self.channels * 2];
            self.decode_block(rest, sub_block, order, &mut pcm);
            out.write_all(&pcm).map_err(AudioError::Write)?;
        }
        Ok(())
    }

    fn decode_block(self, block: &[u8], sub_block: usize, order: ByteOrder, pcm: &mut [u8]) {
        let samples = self.samples(sub_block);
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
            match self.codec {
                Codec::PtAdpcm => ptadpcm::decode(sub, out),
                _ => ima::decode(sub, order, out),
            }
        }
    }

    fn partial_sub_block(self, rest: usize) -> Option<usize> {
        let sub_block = rest / self.channels;
        (self.codec == Codec::ImaAdpcm && sub_block > ima::HEADER_LEN).then_some(sub_block)
    }

    const fn samples(self, sub_block: usize) -> usize {
        match self.codec {
            Codec::PtAdpcm => ptadpcm::samples_per_frame(sub_block),
            _ => ima::samples_per_sub_block(sub_block),
        }
    }
}

const fn header_len(codec: Codec) -> Option<usize> {
    match codec {
        Codec::ImaAdpcm => Some(ima::HEADER_LEN),
        Codec::PtAdpcm => Some(ptadpcm::HEADER_LEN),
        _ => None,
    }
}
