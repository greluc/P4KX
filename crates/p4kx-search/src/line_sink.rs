// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::excerpt::Excerpt;
use crate::{EntryHit, Needle};

const MAX_LINE_LEN: usize = 1 << 20;
const STOP_POLL_MASK: u64 = 0x3FF;

/// Why a [`LineSink`] stopped taking data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Halt {
    Binary,
    Done,
    Cancelled,
}

/// What kind of text a [`LineSink`] takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    ConvertedXml,
}

/// Searches the lines of the text written to it, without holding more than one line.
///
/// A line ends at a line feed, a carriage return and line feed, or a lone carriage return; one
/// longer than a mebibyte is searched up to that length. The sink refuses further data once
/// it has found enough, met data that is no text, or been told to stop; [`LineSink::halt`] says
/// which.
pub(crate) struct LineSink<'a> {
    needle: &'a Needle,
    path: &'a str,
    per_item: usize,
    kind: Kind,
    after_cr: bool,
    stop: &'a (dyn Fn() -> bool + Sync),
    pending: Vec<u8>,
    overlong: bool,
    pending_len: u64,
    line_start: u64,
    line: u64,
    hits: Vec<EntryHit>,
    halt: Option<Halt>,
}

impl<'a> LineSink<'a> {
    pub(crate) const fn new(
        needle: &'a Needle,
        path: &'a str,
        per_item: usize,
        stop: &'a (dyn Fn() -> bool + Sync),
    ) -> Self {
        Self {
            needle,
            path,
            per_item,
            kind: Kind::Text,
            after_cr: false,
            stop,
            pending: Vec::new(),
            overlong: false,
            pending_len: 0,
            line_start: 0,
            line: 0,
            hits: Vec::new(),
            halt: None,
        }
    }

    /// Takes text from `CryXml` converted to XML: control characters are no sign of binary data,
    /// and only a line feed ends a line, as in the XML view.
    pub(crate) const fn accepting_controls(mut self) -> Self {
        self.kind = Kind::ConvertedXml;
        self
    }

    pub(crate) const fn halt(&self) -> Option<Halt> {
        self.halt
    }

    /// Searches the last line when the text does not end with a line break and returns the hits.
    pub(crate) fn finish(mut self) -> (Vec<EntryHit>, Option<Halt>) {
        if self.halt.is_none() && !self.pending.is_empty() {
            let _ended = self.complete();
        }
        (self.hits, self.halt)
    }

    fn stopped(&mut self, halt: Halt) -> io::Error {
        self.halt = Some(halt);
        io::Error::other("the search of the Entry ended")
    }

    fn append(&mut self, bytes: &[u8]) {
        self.pending_len += bytes.len() as u64;
        let room = MAX_LINE_LEN.saturating_sub(self.pending.len());
        if bytes.len() > room {
            self.overlong = true;
        }
        self.pending
            .extend_from_slice(bytes.get(..room.min(bytes.len())).unwrap_or_default());
    }

    fn complete(&mut self) -> io::Result<()> {
        let pending = std::mem::take(&mut self.pending);
        let result = self.search(&pending, self.overlong);
        self.pending = pending;
        self.pending.clear();
        self.pending_len = 0;
        self.overlong = false;
        result
    }

    fn search(&mut self, bytes: &[u8], truncated: bool) -> io::Result<()> {
        let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
        if self.kind == Kind::Text && bytes.iter().any(|&byte| is_binary(byte)) {
            return Err(self.stopped(Halt::Binary));
        }
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) if truncated && error.error_len().is_none() => {
                std::str::from_utf8(bytes.get(..error.valid_up_to()).unwrap_or_default())
                    .unwrap_or_default()
            }
            Err(_) => return Err(self.stopped(Halt::Binary)),
        };
        self.line += 1;
        if let Some(matched) = self.needle.find(text) {
            self.hits.push(EntryHit {
                path: self.path.to_owned(),
                line: self.line,
                offset: self.line_start,
                excerpt: Excerpt::around(text, matched),
            });
            if self.hits.len() >= self.per_item {
                return Err(self.stopped(Halt::Done));
            }
        }
        if self.line & STOP_POLL_MASK == 0 && (self.stop)() {
            return Err(self.stopped(Halt::Cancelled));
        }
        Ok(())
    }
}

impl Write for LineSink<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if let Some(halt) = self.halt {
            return Err(self.stopped(halt));
        }
        let mut rest = buffer;
        if self.after_cr {
            self.after_cr = false;
            if let Some(tail) = rest.strip_prefix(b"\n") {
                rest = tail;
                self.line_start += 1;
            }
        }
        let cr_breaks = self.kind == Kind::Text;
        while let Some(newline) = rest
            .iter()
            .position(|&byte| byte == b'\n' || (cr_breaks && byte == b'\r'))
        {
            let (line, after) = rest.split_at(newline);
            let mut taken = 1;
            if after.first() == Some(&b'\r') {
                match after.get(1) {
                    Some(b'\n') => taken = 2,
                    None => self.after_cr = true,
                    Some(_) => {}
                }
            }
            rest = after.get(taken..).unwrap_or_default();
            let length = line.len() as u64 + self.pending_len;
            let searched = if self.pending.is_empty() {
                self.search(line, false)
            } else {
                self.append(line);
                self.complete()
            };
            self.line_start += length + taken as u64;
            searched?;
        }
        self.append(rest);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Returns whether `byte` is a control character that text does not hold.
pub(crate) const fn is_binary(byte: u8) -> bool {
    (byte < 0x20 && !matches!(byte, b'\t' | b'\n' | 0x0C | b'\r')) || byte == 0x7F
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io::Write as _;

    use super::LineSink;
    use crate::Needle;

    fn places(writes: &[&[u8]]) -> Result<Vec<(u64, u64)>, Box<dyn Error>> {
        let needle = Needle::new("needle")?;
        let mut sink = LineSink::new(&needle, "a.txt", 10, &|| false);
        for write in writes {
            sink.write_all(write)?;
        }
        let (hits, _) = sink.finish();
        Ok(hits.iter().map(|hit| (hit.line, hit.offset)).collect())
    }

    #[test]
    fn a_carriage_return_and_line_feed_split_between_writes_are_one_line_break()
    -> Result<(), Box<dyn Error>> {
        assert_eq!(
            places(&[b"ab\r", b"\nneedle\r", b"needle\n"])?,
            [(2, 4), (3, 11)],
            "one break after ab, a lone break before the last line"
        );
        Ok(())
    }

    #[test]
    fn a_carriage_return_stays_in_a_line_of_converted_xml() -> Result<(), Box<dyn Error>> {
        let needle = Needle::new("needle")?;
        let mut sink = LineSink::new(&needle, "a.xml", 10, &|| false).accepting_controls();
        sink.write_all(b"a\rb needle\n")?;
        let (hits, _) = sink.finish();
        assert_eq!(
            hits.iter().map(|hit| hit.line).collect::<Vec<_>>(),
            [1],
            "one line"
        );
        Ok(())
    }
}
