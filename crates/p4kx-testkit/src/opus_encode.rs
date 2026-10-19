use opus_pure::{Application, MAX_PACKET_BYTES, OpusEncoder, OpusMSEncoder, Repacketizer};

use crate::opus::TestOpusError;

/// The samples per channel of one 20 ms packet at 48 kHz.
pub(crate) const FRAME: usize = 960;
const RATE: i32 = 48_000;
const BITRATE_PER_CHANNEL: i32 = 96_000;
const VORBIS_ORDER: [&[usize]; 8] = [
    &[0],
    &[0, 1],
    &[0, 2, 1],
    &[0, 1, 2, 3],
    &[0, 2, 1, 3, 4],
    &[0, 2, 1, 5, 3, 4],
    &[0, 2, 1, 6, 5, 3, 4],
    &[0, 2, 1, 7, 5, 6, 3, 4],
];

/// How the channels of a synthetic Opus sound are coded.
pub(crate) enum Encoder {
    /// One mono or stereo stream.
    Single(Box<OpusEncoder>),
    /// The Vorbis surround layout of mapping family 1.
    Surround(Box<OpusMSEncoder>),
    /// One mono stream per channel in channel order, as mapping family 255 codes them.
    Streams {
        encoders: Vec<OpusEncoder>,
        repacketizer: Repacketizer,
        mono: Vec<i16>,
        packet: Vec<u8>,
    },
}

impl Encoder {
    /// Creates the encoder of `channels` channels; one mono stream per channel when `separate`.
    pub(crate) fn new(
        channels: usize,
        separate: bool,
        application: Application,
    ) -> Result<Self, TestOpusError> {
        let bitrate = BITRATE_PER_CHANNEL * i32::try_from(channels).unwrap_or(1);
        if separate {
            let encoders = (0..channels)
                .map(|_| {
                    OpusEncoder::new(RATE, 1, application).map(|mut encoder| {
                        encoder.bitrate_bps = BITRATE_PER_CHANNEL;
                        encoder
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(TestOpusError::encoder)?;
            return Ok(Self::Streams {
                encoders,
                repacketizer: Repacketizer::new(),
                mono: vec![0; FRAME],
                packet: vec![0; MAX_PACKET_BYTES],
            });
        }
        if channels <= 2 {
            let mut encoder =
                OpusEncoder::new(RATE, channels, application).map_err(TestOpusError::encoder)?;
            encoder.bitrate_bps = bitrate;
            return Ok(Self::Single(Box::new(encoder)));
        }
        let mut encoder =
            OpusMSEncoder::new(RATE, channels, 1, application).map_err(TestOpusError::encoder)?;
        encoder.set_bitrate(bitrate);
        Ok(Self::Surround(Box::new(encoder)))
    }

    /// Returns the encoder delay in samples at 48 kHz.
    pub(crate) fn lookahead(&self) -> usize {
        match self {
            Self::Single(encoder) => encoder.lookahead(),
            Self::Surround(encoder) => encoder.streams().first().map_or(0, OpusEncoder::lookahead),
            Self::Streams { encoders, .. } => encoders.first().map_or(0, OpusEncoder::lookahead),
        }
    }

    /// Returns where each channel of the speaker order goes in the encoder's input.
    pub(crate) fn input_order(&self, channels: usize) -> Vec<usize> {
        match self {
            Self::Surround(_) => VORBIS_ORDER
                .get(channels - 1)
                .map_or_else(|| (0..channels).collect(), |order| order.to_vec()),
            _ => (0..channels).collect(),
        }
    }

    /// Encodes one packet of [`FRAME`] interleaved frames and appends it to `out`.
    pub(crate) fn encode(&mut self, frame: &[i16], out: &mut Vec<u8>) -> Result<(), TestOpusError> {
        let mut buffer = vec![0; MAX_PACKET_BYTES * 8];
        let len = match self {
            Self::Single(encoder) => encoder.encode_s16(frame, FRAME, &mut buffer),
            Self::Surround(encoder) => encoder.encode_s16(frame, FRAME, &mut buffer),
            Self::Streams {
                encoders,
                repacketizer,
                mono,
                packet,
            } => {
                let channels = encoders.len();
                for (channel, encoder) in encoders.iter_mut().enumerate() {
                    for (target, source) in mono
                        .iter_mut()
                        .zip(frame.iter().skip(channel).step_by(channels))
                    {
                        *target = *source;
                    }
                    let len = encoder
                        .encode_s16(mono, FRAME, packet)
                        .map_err(TestOpusError::encoder)?;
                    let coded = packet.get(..len).unwrap_or_default();
                    if channel + 1 == channels {
                        out.extend_from_slice(coded);
                    } else {
                        repacketizer.clear();
                        repacketizer.cat(coded).map_err(TestOpusError::encoder)?;
                        repacketizer
                            .out_self_delimited_into(out)
                            .map_err(TestOpusError::encoder)?;
                    }
                }
                return Ok(());
            }
        }
        .map_err(TestOpusError::encoder)?;
        out.extend_from_slice(buffer.get(..len).unwrap_or_default());
        Ok(())
    }
}
