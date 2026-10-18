use super::ogg_read::OggPackets;
use super::{OpusError, framing};

const SWITCH_HEADER_LEN: usize = 8;
const MAX_PACKET_LEN: usize = 1 << 16;

/// Where the packets of an Opus WEM come from.
#[derive(Clone, Copy)]
pub(crate) enum Framing<'a> {
    /// Wwise Opus: packets back to back, their sizes in the `seek` chunk as 16-bit values.
    Table(&'a [u8]),
    /// Switch Opus: each packet after an 8-byte header holding its big-endian size.
    Switch,
    /// A standard Ogg Opus stream, its two header packets first.
    Ogg,
}

/// The audio packets of an Opus WEM in order, each checked against the bounds of the data.
pub(crate) struct Packets<'a> {
    data: &'a [u8],
    framing: Framing<'a>,
    offset: usize,
    sizes: &'a [u8],
    ogg: Option<OggPackets<'a>>,
    index: u64,
}

impl<'a> Packets<'a> {
    /// Starts at the first audio packet of `data`; for an Ogg stream the two header packets are
    /// skipped.
    pub(crate) fn new(data: &'a [u8], framing: Framing<'a>) -> Result<Self, OpusError> {
        let (sizes, ogg) = match framing {
            Framing::Table(sizes) => (sizes, None),
            Framing::Switch => (&[][..], None),
            Framing::Ogg => {
                let mut ogg = OggPackets::new(data);
                for _ in 0..2 {
                    ogg.next_packet()
                        .ok_or(OpusError::InvalidOgg("the header packets are missing"))??;
                }
                (&[][..], Some(ogg))
            }
        };
        Ok(Self {
            data,
            framing,
            offset: 0,
            sizes,
            ogg,
            index: 0,
        })
    }

    /// Returns the index the next packet will have.
    pub(crate) const fn index(&self) -> u64 {
        self.index
    }

    /// Returns the next audio packet, or `None` after the last one.
    pub(crate) fn next_packet(&mut self) -> Option<Result<&[u8], OpusError>> {
        let index = self.index;
        let out_of_bounds = OpusError::PacketOutOfBounds { index };
        let packet = match self.framing {
            Framing::Ogg => match self.ogg.as_mut()?.next_packet()? {
                Ok(packet) => packet,
                Err(error) => return Some(Err(error)),
            },
            Framing::Table(_) => {
                let (size, rest) = self.sizes.split_first_chunk::<2>()?;
                self.sizes = rest;
                let len = usize::from(u16::from_le_bytes(*size));
                let end = self.offset + len;
                let Some(packet) = self.data.get(self.offset..end) else {
                    self.sizes = &[];
                    return Some(Err(out_of_bounds));
                };
                self.offset = end;
                packet
            }
            Framing::Switch => {
                let header = self.data.get(self.offset..)?;
                if header.is_empty() {
                    return None;
                }
                let start = self.offset + SWITCH_HEADER_LEN;
                let packet = header
                    .first_chunk::<4>()
                    .and_then(|size| usize::try_from(u32::from_be_bytes(*size)).ok())
                    .filter(|&len| len <= MAX_PACKET_LEN)
                    .and_then(|len| self.data.get(start..start + len));
                let Some(packet) = packet else {
                    self.offset = self.data.len();
                    return Some(Err(out_of_bounds));
                };
                self.offset = start + packet.len();
                packet
            }
        };
        self.index += 1;
        Some(Ok(packet))
    }
}

/// Checks a multistream packet of `streams` streams and returns its samples per channel.
pub(crate) fn multistream_samples(
    packet: &[u8],
    streams: u8,
    scratch: &mut Vec<u8>,
    index: u64,
) -> Result<u32, OpusError> {
    let invalid = |reason| OpusError::InvalidPacket { index, reason };
    let mut rest = packet;
    let mut samples = None;
    for _ in 1..streams {
        scratch.clear();
        let taken = framing::take_self_delimited(rest, scratch).map_err(invalid)?;
        let stream = framing::packet_samples(rest).map_err(invalid)?;
        if *samples.get_or_insert(stream) != stream {
            return Err(invalid("the streams differ in duration"));
        }
        rest = rest.get(taken..).unwrap_or_default();
    }
    let last = framing::packet_samples(rest).map_err(invalid)?;
    if *samples.get_or_insert(last) != last {
        return Err(invalid("the streams differ in duration"));
    }
    Ok(last)
}
