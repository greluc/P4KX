use std::ops::Range;

use crate::bank_reader::SectionReader;
use crate::soundbank::BankHeader;
use crate::{BankError, BankWarning, HircObject, SectionId, hirc};

const SECTION_HEADER_LEN: usize = 8;
const DIDX_ENTRY_LEN: usize = 12;
const STID_ENTRY_MIN_LEN: usize = 5;
const LAST_UNSUPPORTED_VERSION: u32 = 48;
const UNIQUE_SECTIONS: [SectionId; 5] = [
    SectionId::BKHD,
    SectionId::DIDX,
    SectionId::DATA,
    SectionId::HIRC,
    SectionId::STID,
];

#[derive(Debug, Clone)]
pub(crate) struct SectionRange {
    pub(crate) id: SectionId,
    pub(crate) body: Range<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct MediaRange {
    pub(crate) id: u32,
    pub(crate) offset: u64,
    pub(crate) range: Range<usize>,
}

pub(crate) struct Parsed {
    pub(crate) header: BankHeader,
    pub(crate) sections: Vec<SectionRange>,
    pub(crate) media: Vec<MediaRange>,
    pub(crate) objects: Vec<HircObject>,
    pub(crate) names: Vec<(u32, Range<usize>)>,
    pub(crate) warnings: Vec<BankWarning>,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Parsed, BankError> {
    if bytes.first_chunk::<4>() != Some(&SectionId::BKHD.0) {
        return Err(BankError::NotSoundbank);
    }
    let sections = sections(bytes)?;
    let body = |id: SectionId| {
        sections
            .iter()
            .find(|section| section.id == id)
            .map(|section| {
                (
                    section.body.clone(),
                    bytes.get(section.body.clone()).unwrap_or_default(),
                )
            })
    };
    let (_, header) = body(SectionId::BKHD).ok_or(BankError::NotSoundbank)?;
    let header = parse_header(header)?;
    let mut warnings = Vec::new();
    let media = match (body(SectionId::DIDX), body(SectionId::DATA)) {
        (Some((_, index)), Some((data, _))) => media(index, data, &mut warnings)?,
        (Some((_, index)), None) => {
            warnings.push(BankWarning::MediaWithoutData {
                entries: (index.len() / DIDX_ENTRY_LEN) as u64,
            });
            Vec::new()
        }
        (None, _) => Vec::new(),
    };
    let objects = match body(SectionId::HIRC) {
        Some((range, hirc)) => {
            let hirc = hirc::parse(header.version, hirc, range.start)?;
            if hirc.trailing > 0 {
                warnings.push(BankWarning::TrailingHircBytes {
                    len: hirc.trailing as u64,
                });
            }
            hirc.objects
        }
        None => Vec::new(),
    };
    let names = match body(SectionId::STID) {
        Some((range, stid)) => names(stid, range.start, &mut warnings)?,
        None => Vec::new(),
    };
    Ok(Parsed {
        header,
        sections,
        media,
        objects,
        names,
        warnings,
    })
}

fn sections(bytes: &[u8]) -> Result<Vec<SectionRange>, BankError> {
    let mut sections: Vec<SectionRange> = Vec::new();
    let mut position = 0;
    while position < bytes.len() {
        let rest = bytes.get(position..).unwrap_or_default();
        let mut tag = [b'?'; 4];
        for (target, byte) in tag.iter_mut().zip(rest) {
            *target = *byte;
        }
        let id = SectionId(tag);
        let size = rest
            .get(4..SECTION_HEADER_LEN)
            .and_then(|field| field.first_chunk::<4>())
            .map(|field| u32::from_le_bytes(*field))
            .ok_or_else(|| BankError::truncated(id, rest.len(), SECTION_HEADER_LEN as u64))?;
        let needed = SECTION_HEADER_LEN as u64 + u64::from(size);
        let end = usize::try_from(needed)
            .ok()
            .filter(|&len| len <= rest.len())
            .ok_or_else(|| BankError::truncated(id, rest.len(), needed))?;
        if UNIQUE_SECTIONS.contains(&id) && sections.iter().any(|section| section.id == id) {
            return Err(BankError::DuplicateSection(id));
        }
        sections.push(SectionRange {
            id,
            body: position + SECTION_HEADER_LEN..position + end,
        });
        position += end;
    }
    Ok(sections)
}

fn parse_header(body: &[u8]) -> Result<BankHeader, BankError> {
    let mut reader = SectionReader::new(SectionId::BKHD, body);
    let version = reader.u32()?;
    if version <= LAST_UNSUPPORTED_VERSION || version > u32::from(u16::MAX) {
        return Err(BankError::UnsupportedVersion { version });
    }
    Ok(BankHeader {
        version,
        id: reader.u32()?,
        language_id: reader.u32()?,
    })
}

fn media(
    index: &[u8],
    data: Range<usize>,
    warnings: &mut Vec<BankWarning>,
) -> Result<Vec<MediaRange>, BankError> {
    if !index.len().is_multiple_of(DIDX_ENTRY_LEN) {
        return Err(BankError::invalid(
            SectionId::DIDX,
            "its size is not a multiple of 12 bytes",
        ));
    }
    let data_len = data.len() as u64;
    let mut reader = SectionReader::new(SectionId::DIDX, index);
    let mut media = Vec::with_capacity(index.len() / DIDX_ENTRY_LEN);
    while reader.remaining() > 0 {
        let id = reader.u32()?;
        let offset = u64::from(reader.u32()?);
        let end = offset + u64::from(reader.u32()?);
        let in_data = |at: u64| {
            usize::try_from(at)
                .ok()
                .and_then(|at| data.start.checked_add(at))
                .filter(|&at| at <= data.end)
        };
        let (Some(start), Some(stop)) = (in_data(offset), in_data(end)) else {
            return Err(BankError::MediaOutsideData {
                id,
                offset,
                end,
                data_len,
            });
        };
        media.push(MediaRange {
            id,
            offset,
            range: start..stop,
        });
    }
    media.sort_by_key(|media| media.id);
    media.dedup_by(|later, first| {
        let repeated = later.id == first.id;
        if repeated {
            warnings.push(BankWarning::DuplicateMedia { id: later.id });
        }
        repeated
    });
    Ok(media)
}

fn names(
    body: &[u8],
    base: usize,
    warnings: &mut Vec<BankWarning>,
) -> Result<Vec<(u32, Range<usize>)>, BankError> {
    let mut reader = SectionReader::new(SectionId::STID, body);
    let _string_type = reader.u32()?;
    let count = usize::try_from(reader.u32()?).unwrap_or(usize::MAX);
    if count > reader.remaining() / STID_ENTRY_MIN_LEN {
        return Err(BankError::invalid(
            SectionId::STID,
            "it lists more names than its body holds",
        ));
    }
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let id = reader.u32()?;
        let len = usize::from(reader.u8()?);
        let range = reader.skip(len)?;
        if std::str::from_utf8(body.get(range.clone()).unwrap_or_default()).is_err() {
            warnings.push(BankWarning::NonUtf8BankName { id });
            continue;
        }
        names.push((id, base + range.start..base + range.end));
    }
    Ok(names)
}
