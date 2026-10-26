// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// The most bytes of a log that count for the public patches: its tail. The launcher rotates its
/// logs at 5 MiB.
pub(crate) const PATCH_TAIL_BYTES: u64 = 16 << 20;
/// The longest line looked at for a public patch.
pub(crate) const PATCH_LINE_BYTES: usize = 64 << 10;
/// The longest line looked at for a library folder; a longer one is skipped without being held.
const LIBRARY_LINE_BYTES: usize = 1 << 20;
const READ_BUFFER: usize = 64 << 10;

/// Which part of a log file [`for_each_line`] reads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    /// The whole file, lines up to 1 MiB: for library folders, which a protection depends on.
    Whole,
    /// The last [`PATCH_TAIL_BYTES`] bytes, lines up to [`PATCH_LINE_BYTES`].
    Tail,
}

/// Calls `visit` with whether the line lies in the last [`PATCH_TAIL_BYTES`] bytes of the regular
/// file `path` and its text without the line break.
///
/// A line over the limit of `part` is skipped without being held. When a tail starts inside a
/// line, that partial line is dropped. A file that is missing, is not a regular file or cannot be
/// read yields the lines read so far.
pub(crate) fn for_each_line(path: &Path, part: Part, mut visit: impl FnMut(bool, &[u8])) {
    read_lines(path, part, |start, tail_start, line| {
        visit(start >= tail_start, line);
    });
}

struct Opened {
    file: File,
    length: u64,
    tail_start: u64,
    start: u64,
    skipping: bool,
}

fn open(path: &Path, part: Part) -> Option<Opened> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let mut file = File::open(path).ok()?;
    if !file.metadata().is_ok_and(|opened| opened.is_file()) {
        return None;
    }
    let length = metadata.len();
    let tail_start = length.saturating_sub(PATCH_TAIL_BYTES);
    let (mut start, mut skipping) = (0, false);
    if part == Part::Tail && tail_start > 0 {
        file.seek(SeekFrom::Start(tail_start - 1)).ok()?;
        let mut before = [0_u8; 1];
        file.read_exact(&mut before).ok()?;
        skipping = before != *b"\n";
        start = tail_start;
    }
    Some(Opened {
        file,
        length,
        tail_start,
        start,
        skipping,
    })
}

fn read_lines(path: &Path, part: Part, mut visit: impl FnMut(u64, u64, &[u8])) -> u64 {
    let Some(Opened {
        file,
        length,
        tail_start,
        start,
        mut skipping,
    }) = open(path, part)
    else {
        return 0;
    };
    let max_line = match part {
        Part::Whole => LIBRARY_LINE_BYTES,
        Part::Tail => PATCH_LINE_BYTES,
    };
    let mut reader = BufReader::with_capacity(READ_BUFFER, file.take(length.saturating_sub(start)));
    let mut line: Vec<u8> = Vec::new();
    let mut line_start = start;
    let mut position = start;
    loop {
        let buffer = match reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return tail_start,
        };
        if buffer.is_empty() {
            if !skipping {
                emit(line_start, tail_start, &line, &mut visit);
            }
            return tail_start;
        }
        let newline = buffer.iter().position(|&byte| byte == b'\n');
        let chunk = newline.map_or(buffer, |end| buffer.get(..end).unwrap_or_default());
        let consumed = newline.map_or(buffer.len(), |end| end + 1);
        if !skipping {
            if line.len() + chunk.len() > max_line {
                skipping = true;
                line.clear();
            } else {
                line.extend_from_slice(chunk);
            }
        }
        reader.consume(consumed);
        position += consumed as u64;
        if newline.is_some() {
            if !skipping {
                emit(line_start, tail_start, &line, &mut visit);
            }
            line.clear();
            skipping = false;
            line_start = position;
        }
    }
}

fn emit(start: u64, tail_start: u64, line: &[u8], visit: &mut impl FnMut(u64, u64, &[u8])) {
    if !line.is_empty() {
        visit(start, tail_start, line.strip_suffix(b"\r").unwrap_or(line));
    }
}
