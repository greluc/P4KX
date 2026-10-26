// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// The most bytes read from one log file: its tail. The launcher rotates its logs at 5 MiB.
const MAX_LOG_BYTES: u64 = 16 << 20;
/// The longest line that is looked at; a longer one is skipped without being held in memory.
const MAX_LINE_BYTES: usize = 64 << 10;
const READ_BUFFER: usize = 64 << 10;

/// Calls `visit` with each line of the regular file `path`, without its line break.
///
/// Only the last [`MAX_LOG_BYTES`] bytes are read, the partial first line of such a tail is
/// dropped, and a line longer than [`MAX_LINE_BYTES`] is skipped. A file that is missing, is not
/// a regular file or cannot be read yields the lines read so far.
pub(crate) fn for_each_line(path: &Path, mut visit: impl FnMut(&[u8])) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if !metadata.is_file() {
        return;
    }
    let Ok(mut file) = File::open(path) else {
        return;
    };
    let tail = metadata.len() > MAX_LOG_BYTES;
    if tail
        && file
            .seek(SeekFrom::Start(metadata.len() - MAX_LOG_BYTES))
            .is_err()
    {
        return;
    }
    let mut reader = BufReader::with_capacity(READ_BUFFER, file.take(MAX_LOG_BYTES));
    let mut line: Vec<u8> = Vec::new();
    let mut skipping = tail;
    loop {
        let buffer = match reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        };
        if buffer.is_empty() {
            if !skipping {
                emit(&line, &mut visit);
            }
            return;
        }
        let newline = buffer.iter().position(|&byte| byte == b'\n');
        let chunk = newline.map_or(buffer, |end| buffer.get(..end).unwrap_or_default());
        let consumed = newline.map_or(buffer.len(), |end| end + 1);
        if !skipping {
            if line.len() + chunk.len() > MAX_LINE_BYTES {
                skipping = true;
                line.clear();
            } else {
                line.extend_from_slice(chunk);
            }
        }
        reader.consume(consumed);
        if newline.is_some() {
            if !skipping {
                emit(&line, &mut visit);
            }
            line.clear();
            skipping = false;
        }
    }
}

fn emit(line: &[u8], visit: &mut impl FnMut(&[u8])) {
    if !line.is_empty() {
        visit(line.strip_suffix(b"\r").unwrap_or(line));
    }
}
