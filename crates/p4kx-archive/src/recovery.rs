// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io;

use crate::central_directory::parse_header;
use crate::format_error::FormatError;
use crate::issue::{Directory, IssueLog};
use crate::source::Source;
use crate::{Checksum, Entry, EntryIssue};

/// The most bytes from the end of the file that recovery scans for directory records.
pub const MAX_RECOVERY_SCAN: u64 = 4 << 30;

/// The most Entries recovery admits.
pub const MAX_RECOVERED_ENTRIES: usize = 1 << 24;

/// The most signature hits recovery examines, accepted or not.
const MAX_CANDIDATES: u64 = 1 << 24;

const CHUNK_LEN: usize = 4 << 20;
const CENTRAL_SIGNATURE: [u8; 4] = 0x0201_4B50_u32.to_le_bytes();
const CENTRAL_HEADER_LEN: usize = 46;
const MAX_HEADER_LEN: usize = CENTRAL_HEADER_LEN + 3 * u16::MAX as usize;
const LOCAL_SIGNATURE: [u8; 4] = 0x0403_4B50_u32.to_le_bytes();
const LOCAL_CIG_SIGNATURE: [u8; 4] = 0x1403_4B50_u32.to_le_bytes();
const LOCAL_HEADER_LEN: usize = 30;
const LOCAL_NAME_AT: usize = 26;

/// Rebuilds the Entries from the central-directory records found by scanning the last
/// [`MAX_RECOVERY_SCAN`] bytes of `source` once from front to back.
///
/// A record counts only if it parses, if the local header at its offset lies before it and
/// carries the same name, and if the Entry limit holds. Whatever else is in the window is skipped
/// byte by byte, so one damaged record costs only itself. Returns `None` when no record
/// qualifies.
pub(crate) fn recover(source: &Source) -> Result<Option<Directory>, FormatError> {
    let end = source.len();
    let mut window = Window::new(source);
    let mut pos = end.saturating_sub(MAX_RECOVERY_SCAN);
    let mut entries: Vec<Entry> = Vec::new();
    let mut log = IssueLog::default();
    let mut zip64 = false;
    let mut candidates = 0_u64;
    while pos + CENTRAL_HEADER_LEN as u64 <= end && entries.len() < MAX_RECOVERED_ENTRIES {
        let chunk = window.get(pos, CHUNK_LEN)?;
        let Some(hit) = find_signature(chunk) else {
            pos += chunk
                .len()
                .saturating_sub(CENTRAL_SIGNATURE.len() - 1)
                .max(1) as u64;
            continue;
        };
        let candidate = pos + hit as u64;
        candidates += 1;
        if candidates > MAX_CANDIDATES {
            break;
        }
        match accept(&mut window, candidate, entries.len())? {
            Some(found) => {
                pos = candidate + found.len;
                zip64 |= found.cig;
                log.absorb(found.log);
                entries.push(found.entry);
            }
            None => pos = candidate + 1,
        }
    }
    if entries.is_empty() {
        return Ok(None);
    }
    if !zip64 {
        for entry in &mut entries {
            if let Some(value) = entry.crc32c() {
                entry.checksum = Checksum::Crc32(value);
            }
        }
    }
    log.record_first(EntryIssue::Recovered);
    Ok(Some(Directory { entries, log }))
}

struct Found {
    entry: Entry,
    len: u64,
    cig: bool,
    log: IssueLog,
}

fn accept(
    window: &mut Window<'_>,
    candidate: u64,
    index: usize,
) -> Result<Option<Found>, FormatError> {
    let source = window.source;
    let bytes = window.get(candidate, MAX_HEADER_LEN)?;
    let mut log = IssueLog::default();
    let Ok(header) = parse_header(bytes, candidate, index, &mut log) else {
        return Ok(None);
    };
    let local = header.entry.data_offset;
    if local
        .checked_add(LOCAL_HEADER_LEN as u64 + header.name.len() as u64)
        .is_none_or(|local_end| local_end > candidate)
    {
        return Ok(None);
    }
    let Some(data_start) = local_data_start(source, local, header.name)? else {
        return Ok(None);
    };
    let (entry, len, cig) = (header.entry, header.len as u64, header.cig);
    if data_start
        .checked_add(entry.compressed_size)
        .is_none_or(|data_end| data_end > source.len())
    {
        log.record(index, EntryIssue::DataOutsideFile);
    }
    Ok(Some(Found {
        entry,
        len,
        cig,
        log,
    }))
}

/// Returns where the data begins after the local header at `offset`, or `None` when that is not
/// a local header carrying `name`. The caller has checked that header and name lie in the source.
fn local_data_start(source: &Source, offset: u64, name: &[u8]) -> Result<Option<u64>, FormatError> {
    let mut fixed = [0_u8; LOCAL_HEADER_LEN];
    if let Err(error) = source.read_into(offset, &mut fixed) {
        return unless_eof(error);
    }
    let signature = fixed.first_chunk::<4>();
    if signature != Some(&LOCAL_SIGNATURE) && signature != Some(&LOCAL_CIG_SIGNATURE) {
        return Ok(None);
    }
    let Some((name_len, extra_len)) = local_lengths(&fixed) else {
        return Ok(None);
    };
    if usize::from(name_len) != name.len() {
        return Ok(None);
    }
    let mut stored = vec![0_u8; name.len()];
    if let Err(error) = source.read_into(offset + LOCAL_HEADER_LEN as u64, &mut stored) {
        return unless_eof(error);
    }
    Ok((stored == name)
        .then(|| offset + LOCAL_HEADER_LEN as u64 + u64::from(name_len) + u64::from(extra_len)))
}

fn local_lengths(fixed: &[u8; LOCAL_HEADER_LEN]) -> Option<(u16, u16)> {
    let lengths = fixed.get(LOCAL_NAME_AT..)?.first_chunk::<4>()?;
    let [name_low, name_high, extra_low, extra_high] = *lengths;
    Some((
        u16::from_le_bytes([name_low, name_high]),
        u16::from_le_bytes([extra_low, extra_high]),
    ))
}

fn unless_eof<T>(error: io::Error) -> Result<Option<T>, FormatError> {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        Ok(None)
    } else {
        Err(error.into())
    }
}

fn find_signature(chunk: &[u8]) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = chunk.get(from..)?.iter().position(|&byte| byte == b'P') {
        let at = from + found;
        if chunk.get(at..at + CENTRAL_SIGNATURE.len()) == Some(CENTRAL_SIGNATURE.as_slice()) {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// A reusable read buffer over a part of the source.
struct Window<'a> {
    source: &'a Source,
    buffer: Vec<u8>,
    start: u64,
}

impl<'a> Window<'a> {
    const fn new(source: &'a Source) -> Self {
        Self {
            source,
            buffer: Vec::new(),
            start: 0,
        }
    }

    /// Returns up to `len` bytes at `offset`, fewer at the end of the source.
    fn get(&mut self, offset: u64, len: usize) -> io::Result<&[u8]> {
        let wanted = usize::try_from(self.source.len().saturating_sub(offset))
            .unwrap_or(usize::MAX)
            .min(len);
        let held = offset
            .checked_sub(self.start)
            .and_then(|skip| usize::try_from(skip).ok())
            .and_then(|skip| self.buffer.get(skip..));
        if held.is_none_or(|held| held.len() < wanted) {
            let count = wanted.max(len.min(CHUNK_LEN)).min(
                usize::try_from(self.source.len().saturating_sub(offset)).unwrap_or(usize::MAX),
            );
            self.buffer.resize(count, 0);
            self.source.read_into(offset, &mut self.buffer)?;
            self.start = offset;
        }
        let skip = usize::try_from(offset - self.start).unwrap_or(0);
        Ok(self
            .buffer
            .get(skip..)
            .map_or(&[][..], |held| held.get(..wanted).unwrap_or(held)))
    }
}
