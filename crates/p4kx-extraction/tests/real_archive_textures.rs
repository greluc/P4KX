//! Texture merging, decoding, cubemaps and alpha tails on a real Archive; runs with `--ignored`
//! when `P4KX_DATA_P4K` names one.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::io::{Read, Write};

use p4kx_extraction::{Archive, ConversionError, DdsHeader, Entry, Texture, TextureError};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const TEXTURE_SAMPLE: usize = 400;
const DECODE_SAMPLE: usize = 3000;
const PER_FORMAT: usize = 12;
const CUBEMAP_SAMPLE: usize = 24;
const ALPHA_SAMPLE: usize = 60;

fn ends_with(entry: &Entry, suffix: &str) -> bool {
    entry
        .path()
        .len()
        .checked_sub(suffix.len())
        .and_then(|start| entry.path().get(start..))
        .is_some_and(|end| end.eq_ignore_ascii_case(suffix))
}

fn read_entry(archive: &Archive, path: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path() == path)
        .ok_or_else(|| format!("{path} is missing"))?;
    let mut data = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut data)?;
    Ok(data)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn real_split_cubemaps_merge_face_by_face_into_valid_cubemaps() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let cubemaps: Vec<Texture<'_>> = archive
        .entries()
        .par_iter()
        .filter(|entry| ends_with(entry, "_cm.dds"))
        .filter_map(|entry| Texture::from_archive(&archive, entry.path()).ok())
        .filter(|texture| {
            texture.header().is_cubemap() && texture.part_count().is_ok_and(|parts| parts > 0)
        })
        .collect();
    assert!(!cubemaps.is_empty(), "the Archive has split cubemaps");

    let outcomes: Vec<Result<String, String>> = cubemaps
        .par_iter()
        .take(CUBEMAP_SAMPLE)
        .map(|texture| check_cubemap(&archive, texture).map_err(|error| error.to_string()))
        .collect();

    writeln!(
        std::io::stderr(),
        "{} split cubemaps, {} checked: {outcomes:#?}",
        cubemaps.len(),
        outcomes.len()
    )?;
    assert!(outcomes.iter().all(Result::is_ok), "every cubemap merges");
    Ok(())
}

fn check_cubemap(
    archive: &Archive,
    texture: &Texture<'_>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let header = texture.header();
    let mut merged = Vec::new();
    let report = texture.merge_into(&mut merged)?;
    let total: u64 = (0..header.mip_count())
        .filter_map(|level| header.mip_size(level))
        .sum();
    let parsed = DdsHeader::parse(&merged)?;
    if merged.len() as u64 != header.header_len() as u64 + total
        || report.size != merged.len() as u64
        || !parsed.is_cubemap()
        || parsed.part_count(merged.len() as u64)? != 0
    {
        return Err(format!("{}: not a complete cubemap", texture.name()).into());
    }
    let largest = read_entry(archive, &format!("{}.{}", texture.name(), report.parts))?;
    let face = usize::try_from(header.surface_size(0).unwrap_or_default())?;
    let pitch: usize = (0..header.mip_count())
        .filter_map(|level| header.surface_size(level))
        .map(usize::try_from)
        .sum::<Result<usize, _>>()?;
    for index in 0..6 {
        let start = header.header_len() + index * pitch;
        if merged.get(start..start + face) != largest.get(index * face..(index + 1) * face) {
            return Err(format!("{}: face {index} is out of place", texture.name()).into());
        }
    }
    Ok(format!("{} {}", texture.name(), header.format()))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn real_alpha_tails_are_found_merged_and_decoded_into_alpha() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let tails: Vec<&Entry> = archive
        .entries()
        .par_iter()
        .filter(|entry| ends_with(entry, ".dds.a"))
        .collect();
    assert!(!tails.is_empty(), "the Archive has alpha tails");
    let stride = (tails.len() / ALPHA_SAMPLE).max(1);

    let outcomes: Vec<Result<String, String>> = tails
        .par_iter()
        .step_by(stride)
        .map(|tail| check_alpha_tail(&archive, tail).map_err(|error| error.to_string()))
        .collect();

    let mut summary: BTreeMap<&str, usize> = BTreeMap::new();
    for outcome in outcomes.iter().flatten() {
        *summary.entry(outcome.as_str()).or_default() += 1;
    }
    writeln!(
        std::io::stderr(),
        "{} alpha tails, {} sampled: {summary:#?}",
        tails.len(),
        outcomes.len()
    )?;
    let failures: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().err())
        .collect();
    assert!(failures.is_empty(), "alpha tails failed: {failures:#?}");
    Ok(())
}

fn check_alpha_tail(
    archive: &Archive,
    tail: &Entry,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let colour_path = tail
        .path()
        .get(..tail.path().len() - 2)
        .ok_or("a tail path ends in .a")?;
    let texture = Texture::from_archive(archive, colour_path)?;
    let found = texture
        .alpha_tail()
        .ok_or_else(|| format!("{colour_path}: the alpha tail is not found"))?;
    let (colour, alpha) = (texture.header(), found.header());
    if (alpha.width(), alpha.height()) != (colour.width(), colour.height()) {
        return Err(format!("{colour_path}: the alpha tail has another size").into());
    }
    let mut merged = Vec::new();
    let report = Texture::from_archive(archive, tail.path())?.merge_into(&mut merged)?;
    if report.size != merged.len() as u64 {
        return Err(format!("{}: merged size", tail.path()).into());
    }
    let kind = match texture.decode_mip(0) {
        Ok(image) if (image.width(), image.height()) == (colour.width(), colour.height()) => {
            "decoded with alpha"
        }
        Ok(_) => return Err(format!("{colour_path}: decoded size").into()),
        Err(ConversionError::Texture {
            source: TextureError::UnsupportedDecode(_),
            ..
        }) => "merged, no decoder yet",
        Err(error) => return Err(format!("{colour_path}: {error}").into()),
    };
    Ok(format!("{} + {}: {kind}", colour.format(), alpha.format()))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_real_textures_merges_into_standard_dds_files() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let headers = dds_headers(&archive);
    let stride = (headers.len() / TEXTURE_SAMPLE).max(1);

    let outcomes: Vec<(String, Result<String, String>)> = headers
        .par_iter()
        .step_by(stride)
        .map(|entry| {
            (
                entry.path().to_owned(),
                merge_texture(&archive, entry.path()),
            )
        })
        .collect();

    let mut formats: HashMap<String, usize> = HashMap::new();
    let mut failures = Vec::new();
    for (path, outcome) in &outcomes {
        match outcome {
            Ok(format) => *formats.entry(format.clone()).or_default() += 1,
            Err(reason) => failures.push(format!("{path}: {reason}")),
        }
    }
    let mut summary: Vec<_> = formats.iter().collect();
    summary.sort();
    writeln!(
        std::io::stderr(),
        "{} textures sampled; merged per format or skip reason: {summary:?}",
        outcomes.len()
    )?;
    assert!(
        failures.is_empty(),
        "{} of {} textures failed: {:#?}",
        failures.len(),
        outcomes.len(),
        failures.iter().take(20).collect::<Vec<_>>()
    );
    assert!(
        formats.len() > 3,
        "several formats are sampled: {summary:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_real_textures_of_every_format_decodes_and_exports_png() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let headers = dds_headers(&archive);
    let stride = (headers.len() / DECODE_SAMPLE).max(1);
    let sampled: Vec<Texture<'_>> = headers
        .par_iter()
        .step_by(stride)
        .filter_map(|entry| Texture::from_archive(&archive, entry.path()).ok())
        .collect();
    let mut per_format: HashMap<String, Vec<&Texture<'_>>> = HashMap::new();
    for texture in &sampled {
        let group = per_format
            .entry(texture.header().format().to_string())
            .or_default();
        if group.len() < PER_FORMAT {
            group.push(texture);
        }
    }

    let outcomes: Vec<(String, Result<String, String>)> = per_format
        .par_iter()
        .flat_map(|(format, textures)| textures.par_iter().map(move |texture| (format, texture)))
        .map(|(format, texture)| (format.clone(), decode_texture(texture)))
        .collect();

    let mut summary: HashMap<String, usize> = HashMap::new();
    let mut failures = Vec::new();
    for (format, outcome) in outcomes {
        match outcome {
            Ok(kind) => *summary.entry(format!("{format}: {kind}")).or_default() += 1,
            Err(reason) => failures.push(format!("{format}: {reason}")),
        }
    }
    let mut summary: Vec<_> = summary.into_iter().collect();
    summary.sort();
    writeln!(
        std::io::stderr(),
        "{} textures in {} formats decoded: {summary:#?}",
        sampled.len(),
        per_format.len()
    )?;
    assert!(failures.is_empty(), "decoding failed: {failures:#?}");
    for format in ["BC1_UNORM", "BC3_UNORM", "B8G8R8A8_UNORM"] {
        assert!(
            summary
                .iter()
                .any(|(kind, _)| kind == &format!("{format}: decoded")),
            "{format} is sampled and decodes: {summary:?}"
        );
    }

    export_one_png_per_format(&per_format)
}

fn export_one_png_per_format(per_format: &HashMap<String, Vec<&Texture<'_>>>) -> TestResult {
    let target = tempfile::tempdir()?;
    for (format, textures) in per_format {
        let Some(texture) = textures.iter().find(|texture| {
            let header = texture.header();
            header.format().is_decodable() && header.surfaces() == 1 && header.depth() == 1
        }) else {
            continue;
        };
        let out = target.path().join(format!("{format}.png"));
        let report = texture.png_to_file(&out, 0)?;
        let reader =
            png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&out)?)).read_info()?;
        assert_eq!(
            (reader.info().width, reader.info().height),
            (texture.header().width(), texture.header().height()),
            "{}: the PNG has the largest mip's size",
            texture.name()
        );
        assert_eq!(report.level, 0, "the largest mip");
    }
    Ok(())
}

fn decode_texture(texture: &Texture<'_>) -> Result<String, String> {
    let header = texture.header();
    match texture.decode_mip(0) {
        Ok(image) if (image.width(), image.height()) == (header.width(), header.height()) => {
            Ok("decoded".to_owned())
        }
        Ok(image) => Err(format!(
            "{}: decoded {} x {}, header says {} x {}",
            texture.name(),
            image.width(),
            image.height(),
            header.width(),
            header.height()
        )),
        Err(ConversionError::Texture {
            source: TextureError::UnsupportedDecode(_),
            ..
        }) => Ok("no decoder".to_owned()),
        Err(ConversionError::Texture {
            source: TextureError::UnsupportedImage { images },
            ..
        }) => Ok(format!("skipped: {images} images per mip")),
        Err(error) => Err(format!("{}: {error}", texture.name())),
    }
}

fn dds_headers(archive: &Archive) -> Vec<&Entry> {
    archive
        .entries()
        .iter()
        .filter(|entry| {
            entry
                .path()
                .as_bytes()
                .last_chunk::<4>()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(b".dds"))
        })
        .collect()
}

fn merge_texture(archive: &Archive, path: &str) -> Result<String, String> {
    let texture = Texture::from_archive(archive, path).map_err(|error| error.to_string())?;
    let format = texture.header().format().to_string();
    let mut merged = Vec::new();
    match texture.merge_into(&mut merged) {
        Ok(report) => {
            assert_eq!(report.size, merged.len() as u64, "{path}: reported size");
            let total: u64 = (0..texture.header().mip_count())
                .filter_map(|level| texture.header().mip_size(level))
                .sum();
            assert!(
                merged.len() as u64 >= texture.header().header_len() as u64 + total,
                "{path}: every mip is present"
            );
            Ok(format)
        }
        Err(ConversionError::Texture {
            source: TextureError::UnsupportedFormat(format),
            ..
        }) => Ok(format!("skipped: unsupported {format}")),
        Err(error) => Err(error.to_string()),
    }
}
