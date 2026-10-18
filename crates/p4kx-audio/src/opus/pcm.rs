use std::io::Write;
use std::panic::{self, AssertUnwindSafe};

use opus_pure::OpusDecoder;

use super::framing::{self, MAX_PACKET_SAMPLES};
use super::layout::Layout;
use super::{OPUS_RATE, OpusError, OpusStream};
use crate::{AudioError, WavReport, Wem};

const PANICKED: &str = "the decoder failed";
const SILENT: u8 = 255;
const MAX_FRAMES: usize = MAX_PACKET_SAMPLES as usize;

impl<B: AsRef<[u8]>> Wem<B> {
    /// Decodes an Opus sound and writes it as a 16-bit PCM WAV to `out`.
    pub(crate) fn write_opus_wav(&self, out: &mut impl Write) -> Result<WavReport, AudioError> {
        let stream = self.opus_stream()?;
        let (header, report) = self.decoded_wav_header(stream.frames())?;
        out.write_all(&header).map_err(AudioError::Write)?;
        stream.write_pcm(out)?;
        Ok(report)
    }
}

impl OpusStream<'_> {
    /// Decodes the packets and writes the playable frames as interleaved little-endian 16-bit
    /// samples in the WEM's speaker order.
    fn write_pcm(&self, out: &mut impl Write) -> Result<(), AudioError> {
        let invalid = AudioError::InvalidOpus;
        let mut decoders = Decoders::new(&self.layout).map_err(invalid)?;
        let mut packets = self.packets().map_err(invalid)?;
        let width = self.layout.decoded_channels();
        let mut skip = usize::from(self.pre_skip);
        let mut remaining = self.frames;
        let mut bytes = Vec::new();
        while remaining > 0 {
            let index = packets.index();
            let Some(packet) = packets.next_packet() else {
                break;
            };
            let decoded = decoders
                .decode(packet.map_err(invalid)?, index)
                .map_err(invalid)?;
            let frames = decoded.len() / width;
            let skipped = skip.min(frames);
            skip -= skipped;
            let take = usize::try_from(remaining)
                .map_or(frames - skipped, |left| left.min(frames - skipped));
            bytes.clear();
            for frame in decoded.chunks_exact(width).skip(skipped).take(take) {
                for &channel in &self.layout.wav_order {
                    let sample = if channel == SILENT {
                        0
                    } else {
                        frame.get(usize::from(channel)).copied().unwrap_or_default()
                    };
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
            }
            out.write_all(&bytes).map_err(AudioError::Write)?;
            remaining -= take as u64;
        }
        if remaining > 0 {
            return Err(invalid(OpusError::MissingSamples {
                expected: self.frames,
                decoded: self.frames - remaining,
            }));
        }
        Ok(())
    }
}

/// One Opus decoder per stream of a packet, and the buffers they decode into.
struct Decoders {
    streams: Vec<OpusDecoder>,
    coupled: usize,
    width: usize,
    stream_pcm: Vec<i16>,
    pcm: Vec<i16>,
    packet: Vec<u8>,
}

impl Decoders {
    fn new(layout: &Layout) -> Result<Self, OpusError> {
        let coupled = usize::from(layout.coupled);
        let decoders = (0..usize::from(layout.streams))
            .map(|stream| {
                let channels = if stream < coupled { 2 } else { 1 };
                OpusDecoder::new(OPUS_RATE.cast_signed(), channels)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| OpusError::Undecodable {
                index: 0,
                reason: error.to_string(),
            })?;
        let width = layout.decoded_channels();
        Ok(Self {
            streams: decoders,
            coupled,
            width,
            stream_pcm: vec![0; MAX_FRAMES * 2],
            pcm: vec![0; MAX_FRAMES * width],
            packet: Vec::new(),
        })
    }

    /// Decodes a packet and returns its frames, interleaved in the order of the streams.
    fn decode(&mut self, packet: &[u8], index: u64) -> Result<&[i16], OpusError> {
        let Self {
            streams: decoders,
            coupled,
            width,
            stream_pcm,
            pcm,
            packet: scratch,
        } = self;
        let streams = decoders.len();
        let mut rest = packet;
        let mut frames = None;
        let mut channel = 0;
        for (stream, decoder) in decoders.iter_mut().enumerate() {
            let data = if stream + 1 == streams {
                rest
            } else {
                scratch.clear();
                let taken = framing::take_self_delimited(rest, scratch)
                    .map_err(|reason| OpusError::InvalidPacket { index, reason })?;
                rest = rest.get(taken..).unwrap_or_default();
                scratch.as_slice()
            };
            let undecodable = |reason: String| OpusError::Undecodable { index, reason };
            let decoded = panic::catch_unwind(AssertUnwindSafe(|| {
                decoder
                    .decode_s16(data, MAX_FRAMES, stream_pcm)
                    .map_err(|error| error.to_string())
            }))
            .unwrap_or_else(|_| Err(PANICKED.to_owned()))
            .map_err(undecodable)?;
            if *frames.get_or_insert(decoded) != decoded {
                return Err(undecodable("the streams differ in duration".to_owned()));
            }
            let channels = if stream < *coupled { 2 } else { 1 };
            let end = channel + channels;
            for (target, source) in pcm
                .chunks_exact_mut(*width)
                .zip(stream_pcm.chunks_exact(channels))
                .take(decoded)
            {
                if let Some(target) = target.get_mut(channel..end) {
                    target.copy_from_slice(source);
                }
            }
            channel = end;
        }
        let len = frames.unwrap_or_default() * *width;
        Ok(pcm.get(..len).unwrap_or_default())
    }
}
