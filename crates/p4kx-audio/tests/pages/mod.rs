//! An independent reader of Ogg pages for checking what the crate writes.

/// One Ogg page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Page {
    pub(crate) flags: u8,
    pub(crate) granule: u64,
    pub(crate) serial: u32,
    pub(crate) sequence: u32,
    pub(crate) lacing: Vec<u8>,
    pub(crate) body: Vec<u8>,
}

impl Page {
    pub(crate) const fn continued(&self) -> bool {
        self.flags & 0x01 != 0
    }

    pub(crate) const fn first(&self) -> bool {
        self.flags & 0x02 != 0
    }

    pub(crate) const fn last(&self) -> bool {
        self.flags & 0x04 != 0
    }
}

/// Splits `bytes` into pages, checking the capture pattern, the version and the CRC of each.
pub(crate) fn pages(bytes: &[u8]) -> Result<Vec<Page>, String> {
    let mut pages = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let header = rest.get(..27).ok_or("truncated page header")?;
        if header.get(..4) != Some(b"OggS".as_slice()) || header.get(4) != Some(&0) {
            return Err("no Ogg page".to_owned());
        }
        let field = |range: std::ops::Range<usize>| header.get(range).unwrap_or_default();
        let segments = usize::from(header.get(26).copied().unwrap_or_default());
        let lacing = rest
            .get(27..27 + segments)
            .ok_or("truncated lacing")?
            .to_vec();
        let body_len: usize = lacing.iter().map(|&value| usize::from(value)).sum();
        let len = 27 + segments + body_len;
        let page = rest.get(..len).ok_or("truncated body")?;
        let mut zeroed = page.to_vec();
        zeroed.get_mut(22..26).ok_or("short")?.fill(0);
        let stored = u32::from_le_bytes(field(22..26).try_into().map_err(|_| "crc")?);
        if crc(&zeroed) != stored {
            return Err(format!("bad CRC on page {}", pages.len()));
        }
        pages.push(Page {
            flags: header.get(5).copied().unwrap_or_default(),
            granule: u64::from_le_bytes(field(6..14).try_into().map_err(|_| "granule")?),
            serial: u32::from_le_bytes(field(14..18).try_into().map_err(|_| "serial")?),
            sequence: u32::from_le_bytes(field(18..22).try_into().map_err(|_| "sequence")?),
            lacing,
            body: page.get(27 + segments..).unwrap_or_default().to_vec(),
        });
        rest = rest.get(len..).unwrap_or_default();
    }
    Ok(pages)
}

/// Joins the pages' segments into packets, each with the granule of the page it ends on.
pub(crate) fn packets(pages: &[Page]) -> Vec<(Vec<u8>, u64)> {
    let mut packets = Vec::new();
    let mut current = Vec::new();
    for page in pages {
        let mut offset = 0;
        for &value in &page.lacing {
            let end = offset + usize::from(value);
            current.extend_from_slice(page.body.get(offset..end).unwrap_or_default());
            offset = end;
            if value < 255 {
                packets.push((std::mem::take(&mut current), page.granule));
            }
        }
    }
    packets
}

fn crc(bytes: &[u8]) -> u32 {
    let mut crc = 0_u32;
    for &byte in bytes {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x04C1_1DB7
            };
        }
    }
    crc
}
