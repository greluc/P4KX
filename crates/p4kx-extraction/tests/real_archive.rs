//! Extraction, `CryXml` and texture Conversion on a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_extraction::cryxml::{self, Format};
use p4kx_extraction::{
    Archive, ConflictPolicy, ConversionError, DataForgeEntry, DataForgeError, Entry,
    ExtractOptions, Texture, TextureError, extract,
};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;
type SendError = Box<dyn Error + Send + Sync>;

const PER_KIND: usize = 10;
const MAX_SIZE: u64 = 4 << 20;
const CRYXML_EXTENSIONS: [&str; 2] = [".xml", ".mtl"];
const MAX_CRYXML_SIZE: u64 = 64 << 20;
const TEXTURE_SAMPLE: usize = 400;
const CONVERSION_SAMPLE: usize = 2000;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn converts_every_cryxml_entry_of_a_real_archive() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.uncompressed_size() <= MAX_CRYXML_SIZE)
        .filter(|entry| {
            let path = entry.path().to_ascii_lowercase();
            CRYXML_EXTENSIONS
                .iter()
                .any(|extension| path.ends_with(extension))
        })
        .collect();
    let binary = AtomicUsize::new(0);
    let text = AtomicUsize::new(0);

    let failures: Vec<String> = candidates
        .par_iter()
        .filter_map(|entry| match convert(&archive, entry) {
            Ok(Some(Format::CryXml(_))) => {
                binary.fetch_add(1, Ordering::Relaxed);
                None
            }
            Ok(Some(Format::Text)) => {
                text.fetch_add(1, Ordering::Relaxed);
                None
            }
            Ok(None) => None,
            Err(error) => Some(format!("{}: {error}", entry.path())),
        })
        .collect();

    assert!(
        failures.is_empty(),
        "{} of {} Entries fail, e.g. {:?}",
        failures.len(),
        candidates.len(),
        failures.iter().take(10).collect::<Vec<_>>()
    );
    let (binary, text) = (binary.into_inner(), text.into_inner());
    assert!(
        binary > 1000,
        "converted {binary} CryXml and passed {text} text Entries through"
    );
    Ok(())
}

fn convert(archive: &Archive, entry: &Entry) -> Result<Option<Format>, SendError> {
    let mut bytes = Vec::with_capacity(usize::try_from(entry.uncompressed_size())?);
    archive.open_entry(entry)?.read_to_end(&mut bytes)?;
    let Some(format) = cryxml::detect(&bytes) else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(bytes.len() * 2);
    cryxml::read(&bytes)?.write_to(&mut out)?;
    if format == Format::Text {
        if out != bytes {
            return Err("text XML changed".into());
        }
    } else {
        let xml = std::str::from_utf8(&out)?;
        if xml.contains('\r') {
            return Err("XML has a CR".into());
        }
        roxmltree::Document::parse(xml)?;
    }
    Ok(Some(format))
}

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
fn extracts_a_sample_of_real_xml_entries_with_conversion() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let mut seen = HashSet::new();
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.uncompressed_size() <= MAX_SIZE)
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".xml"))
        .filter(|entry| seen.insert(entry.path().replace('/', "\\").to_lowercase()))
        .collect();
    let stride = (candidates.len() / CONVERSION_SAMPLE).max(1);
    let sample: Vec<&Entry> = candidates.into_iter().step_by(stride).collect();
    let target = tempfile::tempdir()?;
    let options = ExtractOptions {
        convert: true,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &sample, target.path(), &options);

    assert!(report.is_complete(), "failed: {:?}", report.failures());
    assert_eq!(
        report.extracted(),
        sample.len(),
        "the whole sample is written"
    );
    assert!(report.converted() > Some(0), "some Entries are CryXml");
    for entry in &sample {
        let written = std::fs::read(target.path().join(entry.path().replace('\\', "/")))?;
        let xml =
            std::str::from_utf8(&written).map_err(|error| format!("{}: {error}", entry.path()))?;
        roxmltree::Document::parse(xml).map_err(|error| format!("{}: {error}", entry.path()))?;
    }

    let options = ExtractOptions {
        policy: ConflictPolicy::IfChanged,
        convert: true,
        ..ExtractOptions::default()
    };
    let again = extract(&archive, &sample, target.path(), &options);

    assert_eq!(
        again.skipped().len(),
        sample.len(),
        "converted files are kept"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn reads_the_dataforge_of_a_real_archive() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;

    let entry = DataForgeEntry::read(&archive)?;

    assert!(
        entry.path().eq_ignore_ascii_case("Data\\Game2.dcb"),
        "Game2.dcb is preferred: {}",
        entry.path()
    );
    match entry.open() {
        Ok(forge) => {
            assert!(forge.record_count() > 0, "the DataForge has Records");
            for record in forge.records() {
                record.name()?;
                record.type_name()?;
                let path = record.record_path()?;
                assert!(
                    forge
                        .records_at_path(path)
                        .any(|found| found.guid() == record.guid()),
                    "{path} finds its Record"
                );
                assert!(
                    forge.record_by_guid(record.guid()).is_some(),
                    "{} finds its Record",
                    record.guid()
                );
            }
        }
        Err(DataForgeError::UnsupportedVersion { version }) => {
            assert!(version > 6, "only a newer version is rejected: {version}");
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_real_textures_merges_into_standard_dds_files() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let headers: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().as_bytes();
            path.len() > 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".dds")
        })
        .collect();
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
