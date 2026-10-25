// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

const RIFF_HEAD_LEN: usize = 12;
const CHUNK_HEAD_LEN: usize = 8;
const MAX_HEADER_LEN: usize = 4096;

/// A sink for the WAV a decoder writes that keeps only a window of its frames.
///
/// It reads the WAV header as it arrives, drops the frames before `from`, keeps those up to `to`
/// and, once the window is full and `stop_when_full` is set, fails the next write so that the
/// decoder stops. Frames beyond `limit` bytes of PCM mark the sound as too long.
#[derive(Debug)]
pub(crate) struct Window {
    header: Vec<u8>,
    format: Option<Vec<u8>>,
    seen: u64,
    from: u64,
    to: u64,
    limit: u64,
    stop_when_full: bool,
    pcm: Vec<u8>,
    stopped: Option<Stop>,
    in_data: bool,
}

/// Why a [`Window`] ended the decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    Full,
    Exceeded,
}

/// What a [`Window`] kept.
#[derive(Debug)]
pub(crate) struct Kept {
    pub(crate) format: Vec<u8>,
    pub(crate) pcm: Vec<u8>,
    pub(crate) seen: u64,
}

impl Window {
    /// Creates a window over the PCM bytes `from..to`, refusing PCM beyond `limit` bytes.
    pub(crate) const fn new(from: u64, to: u64, limit: u64, stop_when_full: bool) -> Self {
        Self {
            header: Vec::new(),
            format: None,
            seen: 0,
            from,
            to,
            limit,
            stop_when_full,
            pcm: Vec::new(),
            stopped: None,
            in_data: false,
        }
    }

    /// Returns whether the decoder was stopped because the window was full.
    pub(crate) fn is_full(&self) -> bool {
        self.stopped == Some(Stop::Full)
    }

    /// Returns whether the sound has more PCM than the limit allows.
    pub(crate) fn is_exceeded(&self) -> bool {
        self.stopped == Some(Stop::Exceeded)
    }

    /// Returns what the window kept, or `None` when the decoder wrote no valid WAV header.
    pub(crate) fn into_kept(self) -> Option<Kept> {
        let format = self.format?;
        Some(Kept {
            format,
            pcm: self.pcm,
            seen: self.seen,
        })
    }

    fn header_end(&mut self) -> Option<usize> {
        let mut at = RIFF_HEAD_LEN;
        loop {
            let head = self.header.get(at..at + CHUNK_HEAD_LEN)?;
            let size =
                usize::try_from(u32::from_le_bytes(head.get(4..8)?.try_into().ok()?)).ok()?;
            let id = head.get(..4)?;
            if id == b"data" {
                return Some(at + CHUNK_HEAD_LEN);
            }
            let end = (at + CHUNK_HEAD_LEN).checked_add(size)?;
            if id == b"fmt " {
                self.format = Some(self.header.get(at..end)?.to_vec());
            }
            at = end + (size & 1);
        }
    }

    fn take(&mut self, bytes: &[u8]) -> io::Result<()> {
        let before = self.seen;
        let len = bytes.len() as u64;
        self.seen = before.saturating_add(len);
        let start = self.from.max(before);
        let end = self.to.min(self.seen);
        if start < end
            && let Some(kept) = bytes.get(
                usize::try_from(start - before).unwrap_or(usize::MAX)
                    ..usize::try_from(end - before).unwrap_or(usize::MAX),
            )
        {
            self.pcm.extend_from_slice(kept);
        }
        if self.seen > self.limit && self.seen > self.to {
            self.stopped = Some(Stop::Exceeded);
            return Err(io::Error::other("the sound exceeds the decoding limit"));
        }
        if self.stop_when_full && self.seen >= self.to {
            self.stopped = Some(Stop::Full);
            return Err(io::Error::other("the window is full"));
        }
        Ok(())
    }
}

impl Write for Window {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.in_data {
            self.take(buf)?;
            return Ok(buf.len());
        }
        self.header.extend_from_slice(buf);
        if let Some(end) = self.header_end() {
            let rest = self.header.split_off(end);
            self.in_data = true;
            if self.format.is_none() {
                return Err(io::Error::other("the WAV has no fmt chunk"));
            }
            self.take(&rest)?;
        } else if self.header.len() > MAX_HEADER_LEN {
            return Err(io::Error::other("the WAV header is too long"));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::Window;

    fn wav(frames: u16) -> Vec<u8> {
        let data: Vec<u8> = (0..frames).flat_map(u16::to_le_bytes).collect();
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16_u32.to_le_bytes());
        out.extend_from_slice(&[1, 0, 1, 0, 0xE8, 3, 0, 0, 0xD0, 7, 0, 0, 2, 0, 16, 0]);
        out.extend_from_slice(b"smpl");
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn a_window_keeps_its_frames_however_the_bytes_arrive() -> Result<(), Box<dyn std::error::Error>>
    {
        for piece in [1, 5, 7, 4096] {
            let mut window = Window::new(20, 60, 1 << 20, false);
            for chunk in wav(100).chunks(piece) {
                window.write_all(chunk)?;
            }
            let kept = window.into_kept().ok_or("kept")?;
            assert_eq!(kept.pcm.len(), 40, "piece {piece}: bytes in the window");
            assert_eq!(
                kept.pcm.first(),
                Some(&10),
                "piece {piece}: starts at frame 10"
            );
            assert_eq!(kept.seen, 200, "piece {piece}: every byte was seen");
            assert_eq!(kept.format.len(), 24, "piece {piece}: the fmt chunk");
        }
        Ok(())
    }

    #[test]
    fn a_full_window_stops_the_decoder_and_a_long_sound_is_refused() {
        let mut window = Window::new(0, 20, 1 << 20, true);
        assert!(window.write_all(&wav(100)).is_err(), "stopped");
        assert!(window.is_full() && !window.is_exceeded(), "full");
        let mut long = Window::new(0, 20, 50, false);
        assert!(long.write_all(&wav(100)).is_err(), "refused");
        assert!(long.is_exceeded(), "exceeded");
    }
}
