//! A survey of the quirks of the soundbanks and WEMs of a real Archive: bank sizes and versions,
//! sections that deviate, and the chunks, tags and padding of the RIFF files.

use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::ops::Range;

use p4kx_extraction::{Archive, Entry, Soundbank};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::wem_survey::WemSurvey;
use crate::{TestResult, report};

const MAX_WEM_LEN: u64 = 512 << 20;
const TOP: usize = 10;
const EXAMPLES: usize = 5;
const SECTION_HEADER_LEN: usize = 8;
const DIDX_ENTRY_LEN: usize = 12;
const MIB: usize = 1 << 20;

#[derive(Default)]
struct Survey {
    banks: usize,
    rejected: Vec<String>,
    sizes: Vec<(u64, String)>,
    versions: BTreeMap<u32, usize>,
    trailing_hirc: Vec<String>,
    duplicate_didx: Vec<String>,
    didx_without_data: Vec<String>,
    non_utf8_stid: Vec<String>,
    skipped: usize,
    wems: WemSurvey,
}

impl Survey {
    fn merge(mut self, other: Self) -> Self {
        self.banks += other.banks;
        self.rejected.extend(other.rejected);
        self.sizes.extend(other.sizes);
        for (version, count) in other.versions {
            *self.versions.entry(version).or_default() += count;
        }
        self.trailing_hirc.extend(other.trailing_hirc);
        self.duplicate_didx.extend(other.duplicate_didx);
        self.didx_without_data.extend(other.didx_without_data);
        self.non_utf8_stid.extend(other.non_utf8_stid);
        self.skipped += other.skipped;
        self.wems = self.wems.merge(other.wems);
        self
    }
}

fn has_extension(entry: &Entry, extension: &[u8]) -> bool {
    let path = entry.path().as_bytes();
    path.len() > extension.len()
        && path
            .get(path.len() - extension.len()..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(extension))
}

fn read_all(archive: &Archive, entry: &Entry) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .ok()?
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        *bytes.get(offset..)?.first_chunk::<4>()?,
    ))
}

struct Sections<'a> {
    bytes: &'a [u8],
    list: Vec<([u8; 4], Range<usize>)>,
}

impl<'a> Sections<'a> {
    fn of(bytes: &'a [u8]) -> Self {
        let mut list = Vec::new();
        let mut position = 0;
        while let (Some(tag), Some(size)) = (
            bytes
                .get(position..)
                .and_then(|rest| rest.first_chunk::<4>()),
            u32_at(bytes, position + 4),
        ) {
            let start = position + SECTION_HEADER_LEN;
            let end = start.saturating_add(size as usize).min(bytes.len());
            list.push((*tag, start..end));
            position = end;
        }
        Self { bytes, list }
    }

    fn body(&self, tag: [u8; 4]) -> Option<&'a [u8]> {
        let (_, range) = self.list.iter().find(|(id, _)| *id == tag)?;
        self.bytes.get(range.clone())
    }
}

fn hirc_trailing(hirc: &[u8]) -> usize {
    let mut position = 4;
    for _ in 0..u32_at(hirc, 0).unwrap_or(0) {
        let Some(len) = u32_at(hirc, position + 1) else {
            break;
        };
        position = position.saturating_add(5).saturating_add(len as usize);
    }
    hirc.len().saturating_sub(position)
}

fn has_duplicate_ids(didx: &[u8]) -> bool {
    let mut seen = HashSet::new();
    didx.as_chunks::<DIDX_ENTRY_LEN>()
        .0
        .iter()
        .any(|entry| !seen.insert(u32_at(entry, 0)))
}

fn has_non_utf8_name(stid: &[u8]) -> bool {
    let mut position = 8;
    for _ in 0..u32_at(stid, 4).unwrap_or(0) {
        let Some(&len) = stid.get(position + 4) else {
            return false;
        };
        let name = stid.get(position + 5..position + 5 + usize::from(len));
        if name.is_some_and(|name| std::str::from_utf8(name).is_err()) {
            return true;
        }
        position += 5 + usize::from(len);
    }
    false
}

fn survey_embedded(survey: &mut Survey, path: &str, didx: &[u8], data: &[u8]) {
    for entry in didx.as_chunks::<DIDX_ENTRY_LEN>().0 {
        let (Some(id), Some(offset), Some(len)) =
            (u32_at(entry, 0), u32_at(entry, 4), u32_at(entry, 8))
        else {
            continue;
        };
        let start = offset as usize;
        if let Some(wem) = data.get(start..start.saturating_add(len as usize)) {
            survey.wems.add(&format!("{path} WEM {id}"), wem);
        }
    }
}

fn survey_bank(archive: &Archive, entry: &Entry) -> Survey {
    let mut survey = Survey::default();
    let path = entry.path().to_owned();
    let Some(bytes) = read_all(archive, entry) else {
        survey.rejected.push(format!("{path}: unreadable"));
        return survey;
    };
    survey.banks = 1;
    survey.sizes.push((bytes.len() as u64, path.clone()));
    let sections = Sections::of(&bytes);
    if let Some(version) = sections.body(*b"BKHD").and_then(|bkhd| u32_at(bkhd, 0)) {
        *survey.versions.entry(version).or_default() += 1;
    }
    if let Err(error) = Soundbank::parse(bytes.as_slice()) {
        survey.rejected.push(format!("{path}: {error}"));
    }
    if let Some(trailing) = sections.body(*b"HIRC").map(hirc_trailing)
        && trailing > 0
    {
        survey
            .trailing_hirc
            .push(format!("{path}: {trailing} bytes"));
    }
    let (didx, data) = (sections.body(*b"DIDX"), sections.body(*b"DATA"));
    if let Some(didx) = didx {
        if has_duplicate_ids(didx) {
            survey.duplicate_didx.push(path.clone());
        }
        if data.is_none() {
            survey.didx_without_data.push(path.clone());
        }
    }
    if sections.body(*b"STID").is_some_and(has_non_utf8_name) {
        survey.non_utf8_stid.push(path.clone());
    }
    if let (Some(didx), Some(data)) = (didx, data) {
        survey_embedded(&mut survey, &path, didx, data);
    }
    survey
}

fn survey_loose(archive: &Archive, entry: &Entry) -> Survey {
    let mut survey = Survey::default();
    if entry.uncompressed_size() > MAX_WEM_LEN {
        survey.skipped = 1;
    } else if let Some(bytes) = read_all(archive, entry) {
        survey.wems.add(entry.path(), &bytes);
    }
    survey
}

fn examples(list: &[String]) -> String {
    format!(
        "{} ({:?})",
        list.len(),
        list.iter().take(EXAMPLES).collect::<Vec<_>>()
    )
}

fn report_banks(survey: &Survey) {
    report(&format!(
        "banks {} (rejected by the parser: {})",
        survey.banks,
        examples(&survey.rejected)
    ));
    report(&format!(
        "largest banks {:?}",
        survey.sizes.iter().take(TOP).collect::<Vec<_>>()
    ));
    let above = |limit: usize| {
        survey
            .sizes
            .iter()
            .filter(|(len, _)| *len > limit as u64)
            .count()
    };
    report(&format!(
        "banks above 16 MiB {}, above 64 MiB {}",
        above(16 * MIB),
        above(64 * MIB)
    ));
    report(&format!("bank versions {:?}", survey.versions));
    report(&format!(
        "trailing HIRC bytes {}",
        examples(&survey.trailing_hirc)
    ));
    report(&format!(
        "duplicate DIDX ids {}",
        examples(&survey.duplicate_didx)
    ));
    report(&format!(
        "DIDX without DATA {}",
        examples(&survey.didx_without_data)
    ));
    report(&format!(
        "non-UTF-8 STID names {}",
        examples(&survey.non_utf8_stid)
    ));
}

fn report_wems(survey: &Survey, loose: usize) {
    let wems = &survey.wems;
    report(&format!(
        "WEMs {} ({} loose above the size cap skipped), {loose} loose entries",
        wems.wems, survey.skipped
    ));
    report(&format!("format tags {:?}", wems.tags));
    for (id, paths) in &wems.after_data {
        report(&format!("chunk {id:?} after data in {}", examples(paths)));
    }
    report(&format!(
        "odd-sized chunks before the last: {} (valid next header with pad {}, without pad {}, neither {})",
        examples(&wems.odd_not_last),
        wems.odd_padded,
        wems.odd_unpadded,
        wems.odd_neither
    ));
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn survey_the_soundbanks_and_wems_of_a_real_archive() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entries = archive.entries();
    let banks: Vec<&Entry> = entries
        .iter()
        .filter(|e| has_extension(e, b".bnk"))
        .collect();
    let loose: Vec<&Entry> = entries
        .iter()
        .filter(|e| has_extension(e, b".wem"))
        .collect();
    let in_banks = banks
        .par_iter()
        .map(|entry| survey_bank(&archive, entry))
        .reduce(Survey::default, Survey::merge);
    let on_their_own = loose
        .par_iter()
        .map(|entry| survey_loose(&archive, entry))
        .reduce(Survey::default, Survey::merge);
    let mut survey = in_banks.merge(on_their_own);
    survey.sizes.sort_unstable_by(|a, b| b.cmp(a));
    report_banks(&survey);
    report_wems(&survey, loose.len());
    Ok(())
}
