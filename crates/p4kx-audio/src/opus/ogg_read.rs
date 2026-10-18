use super::OpusError;
use crate::ogg::crc_update;

const HEADER_LEN: usize = 27;
const CRC_RANGE_START: usize = 22;
const CRC_RANGE_END: usize = 26;
const FLAG_CONTINUED: u8 = 0x01;
const FLAG_LAST: u8 = 0x04;
const MAX_PACKET_LEN: usize = 1 << 16;

/// The packets of the single logical Ogg stream a WEM embeds, read page by page with every page's
/// CRC checked; a packet is copied into a reused buffer only.
pub(crate) struct OggPackets<'a> {
    data: &'a [u8],
    next_page: usize,
    lacing: &'a [u8],
    body: &'a [u8],
    segment: usize,
    body_offset: usize,
    serial: Option<u32>,
    last_page: bool,
    packet: Vec<u8>,
    open: bool,
}

impl<'a> OggPackets<'a> {
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            next_page: 0,
            lacing: &[],
            body: &[],
            segment: 0,
            body_offset: 0,
            serial: None,
            last_page: false,
            packet: Vec::new(),
            open: false,
        }
    }

    /// Returns the next packet, or `None` after the last one.
    pub(crate) fn next_packet(&mut self) -> Option<Result<&[u8], OpusError>> {
        if !self.open {
            self.packet.clear();
        }
        loop {
            while let Some(&value) = self.lacing.get(self.segment) {
                let len = usize::from(value);
                let end = self.body_offset + len;
                let Some(segment) = self.body.get(self.body_offset..end) else {
                    return Some(Err(OpusError::InvalidOgg("a page body is truncated")));
                };
                if self.packet.len() + len > MAX_PACKET_LEN {
                    return Some(Err(OpusError::InvalidOgg("a packet exceeds 64 KiB")));
                }
                self.packet.extend_from_slice(segment);
                self.segment += 1;
                self.body_offset = end;
                self.open = value == 255;
                if !self.open {
                    return Some(Ok(&self.packet));
                }
            }
            if self.last_page || self.next_page >= self.data.len() {
                return self
                    .open
                    .then_some(Err(OpusError::InvalidOgg("the last packet is unfinished")));
            }
            if let Err(error) = self.read_page() {
                self.next_page = self.data.len();
                self.lacing = &[];
                self.open = false;
                return Some(Err(error));
            }
        }
    }

    fn read_page(&mut self) -> Result<(), OpusError> {
        let rest = self.data.get(self.next_page..).unwrap_or_default();
        let header = rest
            .get(..HEADER_LEN)
            .ok_or(OpusError::InvalidOgg("a page header is truncated"))?;
        if header.get(..5) != Some(b"OggS\0".as_slice()) {
            return Err(OpusError::InvalidOgg(
                "a page lacks the OggS capture pattern",
            ));
        }
        let flags = header.get(5).copied().unwrap_or_default();
        let field = |offset: usize| {
            header
                .get(offset..)
                .and_then(<[u8]>::first_chunk::<4>)
                .map_or(0, |bytes| u32::from_le_bytes(*bytes))
        };
        let serial = field(14);
        if *self.serial.get_or_insert(serial) != serial {
            return Err(OpusError::InvalidOgg(
                "the data holds more than one logical stream",
            ));
        }
        let segments = usize::from(header.get(HEADER_LEN - 1).copied().unwrap_or_default());
        let lacing = rest
            .get(HEADER_LEN..HEADER_LEN + segments)
            .ok_or(OpusError::InvalidOgg("a page's lacing is truncated"))?;
        let body_len: usize = lacing.iter().map(|&value| usize::from(value)).sum();
        let page_len = HEADER_LEN + segments + body_len;
        let page = rest
            .get(..page_len)
            .ok_or(OpusError::InvalidOgg("a page body is truncated"))?;
        let crc = [
            page.get(..CRC_RANGE_START).unwrap_or_default(),
            &[0; 4],
            page.get(CRC_RANGE_END..).unwrap_or_default(),
        ]
        .iter()
        .fold(0, |crc, part| crc_update(crc, part));
        if crc != field(CRC_RANGE_START) {
            return Err(OpusError::InvalidOgg("a page's CRC does not match"));
        }
        if (flags & FLAG_CONTINUED != 0) != self.open {
            return Err(OpusError::InvalidOgg("a page's continuation flag is wrong"));
        }
        self.lacing = lacing;
        self.body = page.get(HEADER_LEN + segments..).unwrap_or_default();
        self.segment = 0;
        self.body_offset = 0;
        self.last_page = flags & FLAG_LAST != 0;
        self.next_page += page_len;
        Ok(())
    }
}
