/// The most samples per channel an Opus packet holds: 120 ms at 48 kHz.
pub(crate) const MAX_PACKET_SAMPLES: u32 = 5760;
const MAX_FRAME_LEN: usize = 1275;
const MAX_FRAMES: u8 = 48;

/// Returns the samples per channel at 48 kHz of a normal or self-delimited Opus packet, read
/// from its TOC byte and frame count.
pub(crate) fn packet_samples(packet: &[u8]) -> Result<u32, &'static str> {
    let &toc = packet.first().ok_or("the packet is empty")?;
    let frames = match toc & 0x03 {
        0 => 1,
        1 | 2 => 2,
        _ => u32::from(packet.get(1).ok_or("the frame count is missing")? & 0x3F),
    };
    if frames == 0 {
        return Err("the packet holds no frame");
    }
    let samples = frames * frame_samples(toc);
    if samples > MAX_PACKET_SAMPLES {
        return Err("the packet is longer than 120 ms");
    }
    Ok(samples)
}

const fn frame_samples(toc: u8) -> u32 {
    let size = (toc >> 3) & 0x03;
    if toc & 0x80 != 0 {
        120 << size
    } else if toc & 0x60 == 0x60 {
        if toc & 0x08 == 0 { 480 } else { 960 }
    } else if size == 3 {
        2880
    } else {
        480 << size
    }
}

/// Takes the self-delimited packet at the start of `data`, as RFC 6716 appendix B frames the
/// streams of a multistream packet but the last: appends it to `out` in normal framing and
/// returns the bytes it spans in `data`.
pub(crate) fn take_self_delimited(data: &[u8], out: &mut Vec<u8>) -> Result<usize, &'static str> {
    packet_samples(data)?;
    let mut cursor = Cursor { data, position: 1 };
    let toc = data.first().copied().unwrap_or_default();
    let mut frames_len = 0_usize;
    let mut padding = 0_usize;
    let (count, constant) = match toc & 0x03 {
        0 => (1, true),
        1 => (2, true),
        2 => {
            frames_len = cursor.frame_len()?;
            (2, false)
        }
        _ => {
            let header = cursor.byte()?;
            let count = header & 0x3F;
            if count > MAX_FRAMES {
                return Err("the packet holds more than 48 frames");
            }
            if header & 0x40 != 0 {
                loop {
                    let value = cursor.byte()?;
                    padding += usize::from(value.min(254));
                    if value != 255 {
                        break;
                    }
                }
            }
            let variable = header & 0x80 != 0;
            if variable {
                for _ in 1..count {
                    frames_len += cursor.frame_len()?;
                }
            }
            (count, !variable)
        }
    };
    let length_start = cursor.position;
    let last = cursor.frame_len()?;
    frames_len += if constant {
        last * usize::from(count)
    } else {
        last
    };
    let end = cursor
        .position
        .checked_add(frames_len)
        .and_then(|end| end.checked_add(padding))
        .filter(|&end| end <= data.len())
        .ok_or("the self-delimited packet runs past the multistream packet")?;
    out.extend_from_slice(data.get(..length_start).unwrap_or_default());
    out.extend_from_slice(data.get(cursor.position..end).unwrap_or_default());
    Ok(end)
}

struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl Cursor<'_> {
    fn byte(&mut self) -> Result<u8, &'static str> {
        let value = *self
            .data
            .get(self.position)
            .ok_or("the packet header is truncated")?;
        self.position += 1;
        Ok(value)
    }

    fn frame_len(&mut self) -> Result<usize, &'static str> {
        let first = self.byte()?;
        let len = if first < 252 {
            usize::from(first)
        } else {
            usize::from(first) + 4 * usize::from(self.byte()?)
        };
        if len > MAX_FRAME_LEN {
            return Err("a frame is longer than 1275 bytes");
        }
        Ok(len)
    }
}
