//! Extraction from a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::Write;

use p4kx_extraction::{
    Archive, ConflictPolicy, ConversionError, Entry, ExtractOptions, Texture, TextureError, extract,
};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const PER_KIND: usize = 10;
const MAX_SIZE: u64 = 4 << 20;
const TEXTURE_SAMPLE: usize = 400;
const DECODE_SAMPLE: usize = 3000;
const PER_FORMAT: usize = 12;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn extracts_a_small_subset_of_a_real_archive() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let subset = small_subset(archive.entries());
    let target = tempfile::tempdir()?;

    let report = extract(&archive, &subset, target.path(), &ExtractOptions::default());

    assert!(
        report.is_complete(),
        "every Entry is written: {:?}",
        report.failures()
    );
    assert_eq!(
        report.extracted(),
        subset.len(),
        "the whole subset is written"
    );
    for entry in &subset {
        let file = target.path().join(entry.path().replace('\\', "/"));
        assert_eq!(
            std::fs::metadata(&file)?.len(),
            entry.uncompressed_size(),
            "{} has its uncompressed size on disk",
            entry.path()
        );
    }

    let options = ExtractOptions {
        policy: ConflictPolicy::IfChanged,
        ..ExtractOptions::default()
    };
    let again = extract(&archive, &subset, target.path(), &options);

    assert_eq!(
        again.skipped().len(),
        subset.len(),
        "unchanged files are kept"
    );
    Ok(())
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
            source: TextureError::UnsupportedLayout { surfaces },
            ..
        }) => Ok(format!("skipped: split with {surfaces} images per mip")),
        Err(ConversionError::Texture {
            source: TextureError::UnsupportedFormat(format),
            ..
        }) => Ok(format!("skipped: unsupported {format}")),
        Err(error) => Err(error.to_string()),
    }
}

fn small_subset(entries: &[Entry]) -> Vec<&Entry> {
    let mut per_kind = HashMap::new();
    let mut seen = HashSet::new();
    entries
        .iter()
        .filter(|entry| entry.uncompressed_size() <= MAX_SIZE)
        .filter(|entry| seen.insert(entry.path().replace('/', "\\").to_lowercase()))
        .filter(|entry| {
            let count = per_kind
                .entry((entry.compression(), entry.is_encrypted()))
                .or_insert(0);
            *count += 1;
            *count <= PER_KIND
        })
        .collect()
}
