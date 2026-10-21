use std::ops::Range;

use p4kx_extraction::{Archive, Language, LocalizationEntry};

use crate::excerpt::Excerpt;
use crate::waves::{Chunk, collect};
use crate::{Found, LocalizationHit, Needle, SearchOptions, SearchStats};

const CHUNK_LEN: usize = 256 << 10;

/// Searches the texts of the `global.ini` of each language of `archive`.
///
/// One language is read, searched and dropped at a time, so memory holds a single file. The lines
/// are searched as the file lists them (a key with a comma suffix is a line of its own), by
/// language in the order of [`Language::ALL`], and the search ends once more than
/// [`SearchOptions::limit`] hits exist. [`SearchOptions::languages`] narrows the languages;
/// [`SearchOptions::filter`] narrows the keys. A language whose file cannot be read is counted.
/// Returns `None` once `stop` has returned `true`.
#[must_use]
pub fn search_localization(
    archive: &Archive,
    needle: &Needle,
    options: &SearchOptions,
    stop: &(dyn Fn() -> bool + Sync),
) -> Option<Found<LocalizationHit>> {
    let held = LocalizationEntry::languages(archive);
    let mut found = Found {
        hits: Vec::new(),
        has_more: false,
        stats: SearchStats::default(),
    };
    let limit = options.hit_limit();
    for language in held {
        if !options.languages.is_empty() && !options.languages.contains(&language) {
            continue;
        }
        if stop() {
            return None;
        }
        let Ok(entry) = LocalizationEntry::read(archive, language) else {
            found.stats.languages_unreadable += 1;
            continue;
        };
        found.stats.languages_scanned += 1;
        let text = entry.text().text();
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let ranges = split(text);
        let part = collect(
            ranges.len(),
            limit.saturating_sub(found.hits.len()),
            stop,
            |task| {
                let range = ranges.get(task).cloned().unwrap_or_default();
                scan(
                    text.get(range).unwrap_or_default(),
                    language,
                    needle,
                    options,
                    stop,
                )
            },
        )?;
        found.stats.merge(&part.stats);
        found.hits.extend(part.hits);
        if part.has_more {
            found.has_more = true;
            break;
        }
    }
    Some(found)
}

fn scan(
    lines: &str,
    language: Language,
    needle: &Needle,
    options: &SearchOptions,
    stop: &(dyn Fn() -> bool + Sync),
) -> Chunk<LocalizationHit> {
    let mut chunk = Chunk::default();
    let limit = options.hit_limit();
    if stop() {
        return chunk;
    }
    for line in lines.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || !options.accepts(key) {
            continue;
        }
        chunk.stats.texts_scanned += 1;
        if let Some(matched) = needle.find(value) {
            chunk.hits.push(LocalizationHit {
                language,
                key: key.to_owned(),
                excerpt: Excerpt::around(value, matched),
            });
            if chunk.hits.len() > limit {
                break;
            }
        }
    }
    chunk
}

fn split(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut ranges = Vec::with_capacity(text.len() / CHUNK_LEN + 1);
    let mut start = 0;
    while start < bytes.len() {
        let target = bytes.len().min(start + CHUNK_LEN);
        let end = bytes
            .get(target..)
            .and_then(|rest| rest.iter().position(|&byte| byte == b'\n'))
            .map_or(bytes.len(), |newline| target + newline + 1);
        ranges.push(start..end);
        start = end;
    }
    ranges
}
