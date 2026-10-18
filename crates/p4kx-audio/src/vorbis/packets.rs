use std::iter::{Filter, Peekable};

use super::VorbisError;
use super::bits::{BitWriter, ilog};
use super::header::Header;
use crate::ByteOrder;

const PACKET_HEADER_LEN: usize = 2;

/// One Wwise packet: its offset in the `data` chunk and its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Packet<'a> {
    pub(crate) offset: usize,
    pub(crate) payload: &'a [u8],
}

/// The packets of a `data` chunk from an offset on, each a 16-bit size and its payload.
#[derive(Debug, Clone)]
pub(crate) struct Packets<'a> {
    data: &'a [u8],
    order: ByteOrder,
    offset: usize,
}

impl<'a> Packets<'a> {
    pub(crate) const fn new(data: &'a [u8], order: ByteOrder, offset: usize) -> Self {
        Self {
            data,
            order,
            offset,
        }
    }
}

impl<'a> Iterator for Packets<'a> {
    type Item = Result<Packet<'a>, VorbisError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }
        let offset = self.offset;
        let payload = self.order.u16_at(self.data, offset).and_then(|size| {
            let start = offset.checked_add(PACKET_HEADER_LEN)?;
            self.data.get(start..start.checked_add(usize::from(size))?)
        });
        let Some(payload) = payload else {
            self.offset = self.data.len();
            return Some(Err(VorbisError::PacketOutOfBounds {
                offset: offset as u64,
            }));
        };
        self.offset = offset + PACKET_HEADER_LEN + payload.len();
        Some(Ok(Packet { offset, payload }))
    }
}

type Item<'a> = Result<Packet<'a>, VorbisError>;
type NonEmpty<'a> = Peekable<Filter<Packets<'a>, fn(&Item<'a>) -> bool>>;

/// A rebuilt audio packet: its offset in the `data` chunk, its granule position and whether it
/// is the last one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rebuilt {
    pub(crate) offset: usize,
    pub(crate) granule: u64,
    pub(crate) last: bool,
}

/// Rebuilds the Wwise audio packets as Vorbis audio packets and counts their samples.
#[derive(Debug)]
pub(crate) struct AudioPackets<'a> {
    packets: NonEmpty<'a>,
    header: &'a Header,
    long_modes: &'a [bool],
    mode_bits: u32,
    previous: Option<bool>,
    granule: u64,
}

impl<'a> AudioPackets<'a> {
    pub(crate) fn new(
        data: &'a [u8],
        order: ByteOrder,
        header: &'a Header,
        long_modes: &'a [bool],
    ) -> Self {
        let not_empty: fn(&Item<'a>) -> bool =
            |item| !matches!(item, Ok(packet) if packet.payload.is_empty());
        let mode_count = u32::try_from(long_modes.len()).unwrap_or(u32::MAX);
        Self {
            packets: Packets::new(data, order, header.audio_offset)
                .filter(not_empty)
                .peekable(),
            header,
            long_modes,
            mode_bits: ilog(mode_count.saturating_sub(1)),
            previous: None,
            granule: 0,
        }
    }

    /// Rebuilds the next packet into `out`; `None` after the last one.
    pub(crate) fn next_into(&mut self, out: &mut Vec<u8>) -> Option<Result<Rebuilt, VorbisError>> {
        let packet = match self.packets.next()? {
            Ok(packet) => packet,
            Err(error) => return Some(Err(error)),
        };
        Some(self.rebuild(packet, out))
    }

    fn rebuild(&mut self, packet: Packet<'_>, out: &mut Vec<u8>) -> Result<Rebuilt, VorbisError> {
        let invalid = |reason| VorbisError::InvalidAudioPacket {
            offset: packet.offset as u64,
            reason,
        };
        let (&first, rest) = packet
            .payload
            .split_first()
            .ok_or_else(|| invalid("the packet is empty"))?;
        let mode_mask = (1_u8 << self.mode_bits) - 1;
        out.clear();
        let long = if self.header.modified_packets {
            let mode = first & mode_mask;
            let long = self
                .is_long(mode)
                .ok_or_else(|| invalid("the mode does not exist"))?;
            let next_long = match self.packets.peek() {
                Some(Ok(next)) => next
                    .payload
                    .first()
                    .and_then(|&byte| self.is_long(byte & mode_mask))
                    .unwrap_or(false),
                _ => false,
            };
            let mut writer = BitWriter::new(out);
            writer.write(0, 1);
            writer.write(u32::from(mode), self.mode_bits);
            if long {
                writer.write(u32::from(self.previous.unwrap_or(false)), 1);
                writer.write(u32::from(next_long), 1);
            }
            writer.write(u32::from(first >> self.mode_bits), 8 - self.mode_bits);
            writer.write_bytes(rest);
            writer.finish();
            long
        } else {
            if first & 1 != 0 {
                return Err(invalid("a header packet stands among the audio packets"));
            }
            out.extend_from_slice(packet.payload);
            self.is_long((first >> 1) & mode_mask)
                .ok_or_else(|| invalid("the mode does not exist"))?
        };
        let block = self.header.block_size(long);
        if let Some(previous) = self.previous {
            let samples = self.header.block_size(previous) / 4 + block / 4;
            self.granule = self
                .granule
                .checked_add(samples)
                .ok_or_else(|| invalid("the granule position overflows"))?;
        }
        self.previous = Some(long);
        Ok(Rebuilt {
            offset: packet.offset,
            granule: self.granule,
            last: self.packets.peek().is_none(),
        })
    }

    fn is_long(&self, mode: u8) -> Option<bool> {
        self.long_modes.get(usize::from(mode)).copied()
    }
}
