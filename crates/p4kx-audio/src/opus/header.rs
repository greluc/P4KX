use super::layout::Layout;
use super::ogg_read::OggPackets;
use super::packets::Framing;
use super::{OPUS_RATE, OpusError};
use crate::ByteOrder;

const TAG_SWITCH: u16 = 0x3039;
const TAG_OGG: u16 = 0x3040;
const TAG_WWISE: u16 = 0x3041;
const SWITCH_FMT_LEN: usize = 0x28;
const WWISE_VERSION: u8 = 1;
const OPUS_HEAD_LEN: usize = 19;
const VENDOR: &[u8] = b"P4K-Extractor";

/// What the `fmt` chunk, the `seek` chunk and an embedded `OpusHead` say about an Opus WEM.
pub(crate) struct Header<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) framing: Framing<'a>,
    pub(crate) layout: Layout,
    pub(crate) pre_skip: Option<u16>,
    pub(crate) declared: Option<u64>,
    pub(crate) head: Option<Vec<u8>>,
}

/// The `fmt` chunk of an Opus WEM in its byte order.
#[derive(Clone, Copy)]
struct Fmt<'b> {
    order: ByteOrder,
    bytes: &'b [u8],
}

impl Fmt<'_> {
    fn u16_at(self, offset: usize) -> Option<u16> {
        self.order.u16_at(self.bytes, offset)
    }

    fn u32_at(self, offset: usize) -> Option<u32> {
        self.order.u32_at(self.bytes, offset)
    }
}

impl<'a> Header<'a> {
    /// Reads the header of an Opus WEM whose `fmt`, `seek` and `data` chunks are given.
    pub(crate) fn read(
        order: ByteOrder,
        fmt: &[u8],
        seek: Option<&'a [u8]>,
        data: &'a [u8],
        channels: u16,
    ) -> Result<Self, OpusError> {
        let fmt = Fmt { order, bytes: fmt };
        let tag = fmt.u16_at(0).unwrap_or_default();
        let unaligned = fmt.u16_at(0x0C) != Some(0) || fmt.u16_at(0x0E) != Some(0);
        if unaligned && tag != TAG_SWITCH {
            return Err(OpusError::InvalidHeader(
                "an Opus fmt chunk declares a block alignment or sample size",
            ));
        }
        match tag {
            TAG_WWISE => Self::wwise(fmt, seek, data, channels),
            TAG_SWITCH => Self::switch(fmt, data, channels),
            TAG_OGG => Self::ogg(fmt, data, channels),
            _ => Err(OpusError::InvalidHeader("unknown Opus format tag")),
        }
    }

    fn wwise(
        fmt: Fmt<'_>,
        seek: Option<&'a [u8]>,
        data: &'a [u8],
        channels: u16,
    ) -> Result<Self, OpusError> {
        let byte = |offset: usize| fmt.bytes.get(offset).copied();
        let (Some(layout), Some(count), Some(pre_skip), Some(version), Some(mapping)) = (
            fmt.u32_at(0x14),
            fmt.u32_at(0x1C),
            fmt.u16_at(0x20),
            byte(0x22),
            byte(0x23),
        ) else {
            return Err(OpusError::InvalidHeader(
                "the Wwise Opus fmt chunk is shorter than 36 bytes",
            ));
        };
        if version != WWISE_VERSION {
            return Err(OpusError::InvalidHeader("unknown Wwise Opus version"));
        }
        let sizes = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(2))
            .and_then(|len| seek?.get(..len))
            .ok_or(OpusError::InvalidHeader(
                "the seek chunk holds fewer packet sizes than the fmt chunk counts",
            ))?;
        Ok(Self {
            data,
            framing: Framing::Table(sizes),
            layout: Layout::wwise(channels, layout, mapping)?,
            pre_skip: Some(pre_skip),
            declared: fmt.u32_at(0x18).map(u64::from),
            head: None,
        })
    }

    fn switch(fmt: Fmt<'_>, data: &'a [u8], channels: u16) -> Result<Self, OpusError> {
        let seek_len = fmt
            .u32_at(0x24)
            .filter(|_| fmt.bytes.len() == SWITCH_FMT_LEN)
            .ok_or(OpusError::InvalidHeader(
                "the Switch Opus fmt chunk is not 40 bytes",
            ))?;
        let data = usize::try_from(seek_len)
            .ok()
            .and_then(|len| data.get(len..))
            .ok_or(OpusError::InvalidHeader(
                "the seek table is larger than the data",
            ))?;
        let declared = fmt
            .u32_at(0x18)
            .filter(|_| fmt.u32_at(4) == Some(OPUS_RATE))
            .map(u64::from);
        Ok(Self {
            data,
            framing: Framing::Switch,
            layout: Layout::wwise(channels, 0, 0)?,
            pre_skip: None,
            declared,
            head: None,
        })
    }

    fn ogg(fmt: Fmt<'_>, data: &'a [u8], channels: u16) -> Result<Self, OpusError> {
        let mut packets = OggPackets::new(data);
        let head = packets
            .next_packet()
            .ok_or(OpusError::InvalidOgg("the OpusHead packet is missing"))??
            .to_vec();
        let (pre_skip, layout) = read_head(&head, channels)?;
        let tags = packets
            .next_packet()
            .ok_or(OpusError::InvalidOgg("the OpusTags packet is missing"))??;
        if !tags.starts_with(b"OpusTags") {
            return Err(OpusError::InvalidOgg("the OpusTags packet is missing"));
        }
        Ok(Self {
            data,
            framing: Framing::Ogg,
            layout,
            pre_skip: Some(pre_skip),
            declared: fmt.u32_at(0x18).map(u64::from),
            head: Some(head),
        })
    }
}

fn read_head(head: &[u8], channels: u16) -> Result<(u16, Layout), OpusError> {
    let invalid = OpusError::InvalidHeader;
    if head.len() < OPUS_HEAD_LEN || !head.starts_with(b"OpusHead") {
        return Err(invalid("the OpusHead packet is malformed"));
    }
    let byte = |offset: usize| head.get(offset).copied().unwrap_or_default();
    if byte(8) & 0xF0 != 0 {
        return Err(invalid("the OpusHead version is not supported"));
    }
    if u16::from(byte(9)) != channels {
        return Err(invalid("the OpusHead channels differ from the fmt chunk"));
    }
    let pre_skip = u16::from_le_bytes([byte(10), byte(11)]);
    let family = byte(18);
    let layout = if family == 0 {
        let coupled = u8::from(channels == 2);
        Layout::new(0, 1, coupled, (0..=coupled).collect())?
    } else {
        let mapping = head
            .get(21..21 + usize::from(channels))
            .ok_or_else(|| invalid("the OpusHead channel mapping is truncated"))?;
        Layout::new(family, byte(19), byte(20), mapping.to_vec())?
    };
    Ok((pre_skip, layout))
}

/// Writes the `OpusHead` packet of a stream of `layout` that skips `pre_skip` samples.
pub(crate) fn head_packet(layout: &Layout, pre_skip: u16, out: &mut Vec<u8>) {
    out.extend_from_slice(b"OpusHead");
    out.push(1);
    out.push(u8::try_from(layout.mapping.len()).unwrap_or(u8::MAX));
    out.extend_from_slice(&pre_skip.to_le_bytes());
    out.extend_from_slice(&OPUS_RATE.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.push(layout.family);
    if layout.family != 0 {
        out.push(layout.streams);
        out.push(layout.coupled);
        out.extend_from_slice(&layout.mapping);
    }
}

/// Writes an `OpusTags` packet that names only the vendor.
pub(crate) fn tags_packet(out: &mut Vec<u8>) {
    out.extend_from_slice(b"OpusTags");
    out.extend_from_slice(
        &u32::try_from(VENDOR.len())
            .unwrap_or_default()
            .to_le_bytes(),
    );
    out.extend_from_slice(VENDOR);
    out.extend_from_slice(&0_u32.to_le_bytes());
}
