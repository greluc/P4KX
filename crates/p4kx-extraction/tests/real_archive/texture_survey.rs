//! Every texture header of the real Archive surveyed, and a sample of every format merged and
//! decoded.

use std::collections::{BTreeMap, HashSet};
use std::io::{self, Read, Write};
use std::time::Instant;

use p4kx_extraction::{Archive, ConversionError, DdsHeader, Entry, Texture, TextureError};
use p4kx_testkit::real_archive_path;
use p4kx_texture::MAX_HEADER_LEN;
use rayon::prelude::*;

use crate::{TestResult, report};

const SAMPLES_PER_FORMAT: usize = 16;
const PREVIEW_SIDE: u32 = 1024;
const CUBE_FACES: u32 = 6;
const CROSS_COLUMNS: u32 = 4;
const CROSS_ROWS: u32 = 3;
const BLOCK_FORMATS: [&str; 6] = [
    "BC1_UNORM",
    "BC3_UNORM",
    "BC4_UNORM",
    "BC5_SNORM",
    "BC6H_UF16",
    "BC7_UNORM_SRGB",
];

struct Counted(u64);

impl Write for Counted {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0 += buffer.len() as u64;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(PartialEq, Eq)]
enum Kind {
    Texture,
    AlphaTail,
}

#[derive(Default)]
struct Limits {
    no_decoder: bool,
    several_images: bool,
}

struct Header {
    path: String,
    format: String,
    kind: Kind,
    cubemap: bool,
    alpha_tail: bool,
    limits: Limits,
    parts: Result<u32, String>,
}

#[derive(Default)]
struct Outcome {
    sampled: usize,
    decoded: usize,
    no_decoder: usize,
    several_images: usize,
}

fn read_header(
    archive: &Archive,
    entry: &Entry,
    tails: &HashSet<String>,
) -> Result<Header, String> {
    let mut start = Vec::with_capacity(MAX_HEADER_LEN);
    archive
        .open_entry(entry)
        .map_err(|error| error.to_string())?
        .take(MAX_HEADER_LEN as u64)
        .read_to_end(&mut start)
        .map_err(|error| error.to_string())?;
    let dds = DdsHeader::parse(&start).map_err(|error| error.to_string())?;
    let path = entry.path();
    let is_tail = path.to_ascii_lowercase().ends_with(".dds.a");
    Ok(Header {
        path: path.to_owned(),
        format: dds.format().to_string(),
        kind: if is_tail {
            Kind::AlphaTail
        } else {
            Kind::Texture
        },
        cubemap: dds.is_cubemap(),
        alpha_tail: tails.contains(&format!("{path}.a").to_ascii_lowercase()),
        limits: Limits {
            no_decoder: !dds.format().is_decodable(),
            several_images: dds.depth() > 1
                || dds.array_size() > 1
                || dds.faces() != 1 && dds.faces() != CUBE_FACES,
        },
        parts: dds
            .part_count(entry.uncompressed_size())
            .map_err(|error| error.to_string()),
    })
}

fn headers(archive: &Archive) -> (Vec<Header>, Vec<String>) {
    let entries: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| Texture::is_header_path(entry.path()))
        .collect();
    let tails: HashSet<String> = entries
        .iter()
        .map(|entry| entry.path().to_ascii_lowercase())
        .filter(|path| path.ends_with(".dds.a"))
        .collect();
    let opened: Vec<Result<Header, String>> = entries
        .par_iter()
        .map(|entry| {
            read_header(archive, entry, &tails)
                .map_err(|error| format!("{}: {error}", entry.path()))
        })
        .collect();
    let mut readable = Vec::new();
    let mut unreadable = Vec::new();
    for header in opened {
        match header {
            Ok(header) => readable.push(header),
            Err(error) => unreadable.push(error),
        }
    }
    (readable, unreadable)
}

fn spread<'a>(headers: impl Iterator<Item = &'a Header>) -> Vec<&'a Header> {
    let all: Vec<&Header> = headers.collect();
    let step = all.len().div_ceil(SAMPLES_PER_FORMAT).max(1);
    all.into_iter().step_by(step).collect()
}

fn preview_level(texture: &Texture<'_>) -> u32 {
    let header = texture.header();
    let side = header.width().max(header.height());
    (0..header.mip_count())
        .find(|&level| (side >> level).max(1) <= PREVIEW_SIDE)
        .unwrap_or_else(|| header.mip_count().saturating_sub(1))
}

const fn unsupported(error: &ConversionError) -> Option<&TextureError> {
    match error {
        ConversionError::Texture {
            source:
                source @ (TextureError::UnsupportedDecode(_) | TextureError::UnsupportedImage { .. }),
            ..
        } => Some(source),
        _ => None,
    }
}

fn check_merge(texture: &Texture<'_>) -> TestResult {
    let dds = *texture.header();
    let mut counted = Counted(0);
    let report = texture.merge_into(&mut counted)?;
    let mips: u64 = (0..dds.mip_count())
        .map(|level| dds.mip_size(level).ok_or("a mip size"))
        .sum::<Result<u64, _>>()?;
    let expected = dds.header_len() as u64 + mips;
    if counted.0 != expected || report.size != expected {
        return Err(format!(
            "{}: merged {} bytes, reported {}, expected {expected}",
            texture.name(),
            counted.0,
            report.size
        )
        .into());
    }
    Ok(())
}

fn check(archive: &Archive, header: &Header, outcome: &mut Outcome) -> TestResult {
    let texture = Texture::from_archive(archive, &header.path)?;
    let dds = *texture.header();
    check_merge(&texture)?;
    outcome.sampled += 1;
    let level = preview_level(&texture);
    match texture.decode_preview(level) {
        Ok(image) => {
            let side = |full: u32| (full >> level).max(1);
            let (columns, rows) = if dds.faces() == CUBE_FACES && dds.array_size() == 1 {
                (CROSS_COLUMNS, CROSS_ROWS)
            } else {
                (1, 1)
            };
            let shape = (side(dds.width()) * columns, side(dds.height()) * rows);
            if (image.width(), image.height()) != shape {
                return Err(format!(
                    "{}: decoded {}x{}, expected {shape:?}",
                    header.path,
                    image.width(),
                    image.height()
                )
                .into());
            }
            outcome.decoded += 1;
        }
        Err(error) => match unsupported(&error) {
            Some(TextureError::UnsupportedDecode(_)) => outcome.no_decoder += 1,
            Some(_) => outcome.several_images += 1,
            None => return Err(format!("{}: {error}", header.path).into()),
        },
    }
    Ok(())
}

fn sampled(headers: &[Header], formats: &BTreeMap<&str, usize>) -> Vec<usize> {
    let mut samples: Vec<&Header> = Vec::new();
    for format in formats.keys() {
        samples.extend(spread(headers.iter().filter(|h| h.format == *format)));
    }
    samples.extend(spread(headers.iter().filter(|header| header.cubemap)));
    samples.extend(spread(headers.iter().filter(|header| header.alpha_tail)));
    samples
        .into_iter()
        .filter_map(|sample| headers.iter().position(|header| header.path == sample.path))
        .collect()
}

fn run<'a>(
    archive: &Archive,
    headers: &'a [Header],
    samples: &[usize],
) -> Result<BTreeMap<&'a str, Outcome>, String> {
    let outcomes: Vec<(&str, Outcome)> = samples
        .par_iter()
        .filter_map(|&position| headers.get(position))
        .map(|header| {
            let mut outcome = Outcome::default();
            check(archive, header, &mut outcome)
                .map(|()| (header.format.as_str(), outcome))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<_, String>>()?;
    let mut per_format: BTreeMap<&str, Outcome> = BTreeMap::new();
    for (format, outcome) in outcomes {
        let total = per_format.entry(format).or_default();
        total.sampled += outcome.sampled;
        total.decoded += outcome.decoded;
        total.no_decoder += outcome.no_decoder;
        total.several_images += outcome.several_images;
    }
    Ok(per_format)
}

fn summarise(headers: &[Header], formats: &BTreeMap<&str, usize>, per: &BTreeMap<&str, Outcome>) {
    let count = |keep: &dyn Fn(&Header) -> bool| headers.iter().filter(|h| keep(h)).count();
    let tails = count(&|header| header.kind == Kind::AlphaTail);
    report(&format!(
        "{} textures, {tails} alpha tails, {} cubemaps, {} textures with an alpha tail; {} \
         formats without a decoder, {} holding several images per mip",
        headers.len() - tails,
        count(&|header| header.cubemap),
        count(&|header| header.alpha_tail),
        count(&|header| header.limits.no_decoder),
        count(&|header| header.limits.several_images),
    ));
    for (format, headers) in formats {
        let outcome = per.get(format);
        report(&format!(
            "{format}: {headers} headers; sampled {}, decoded {}, no decoder {}, several \
             images {}",
            outcome.map_or(0, |outcome| outcome.sampled),
            outcome.map_or(0, |outcome| outcome.decoded),
            outcome.map_or(0, |outcome| outcome.no_decoder),
            outcome.map_or(0, |outcome| outcome.several_images),
        ));
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_every_texture_format_merges_and_decodes_without_a_panic() -> TestResult {
    let started = Instant::now();
    let archive = Archive::open(real_archive_path()?)?;
    let (headers, unreadable) = headers(&archive);
    assert!(unreadable.is_empty(), "unreadable: {unreadable:?}");
    let anomalies: Vec<(&str, &str)> = headers
        .iter()
        .filter_map(|header| Some((header.path.as_str(), header.parts.as_ref().err()?.as_str())))
        .collect();
    assert!(
        anomalies.is_empty(),
        "headers that fit no run of mips: {anomalies:?}"
    );

    let mut formats: BTreeMap<&str, usize> = BTreeMap::new();
    for header in &headers {
        *formats.entry(header.format.as_str()).or_default() += 1;
    }
    let surveyed = started.elapsed();
    let samples = sampled(&headers, &formats);
    let per_format = run(&archive, &headers, &samples)?;
    report(&format!(
        "surveyed in {surveyed:?}, sampled in {:?}",
        started.elapsed().saturating_sub(surveyed)
    ));
    summarise(&headers, &formats, &per_format);

    for name in BLOCK_FORMATS {
        let outcome = per_format
            .get(name)
            .ok_or_else(|| format!("the Archive has no {name} texture"))?;
        assert!(outcome.decoded > 0, "{name} decodes");
        assert_eq!(outcome.no_decoder, 0, "{name} has a decoder");
    }
    Ok(())
}
