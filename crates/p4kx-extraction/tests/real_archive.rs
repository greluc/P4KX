//! Extraction from a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{Read, Write};

use p4kx_extraction::{
    Archive, AudioError, Codec, ConflictPolicy, ConversionError, Entry, ExtractOptions, Sound,
    Texture, TextureError, Wem, extract,
};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const PER_KIND: usize = 10;
const MAX_SIZE: u64 = 4 << 20;
const TEXTURE_SAMPLE: usize = 400;
const SOUND_SAMPLE: usize = 2_000;
const SERVICE_SAMPLE: usize = 5;
const MAX_SOUND_SECONDS: u64 = 3_600;

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

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_real_wems_of_every_codec_is_described_and_converted_where_supported() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let wems: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().as_bytes();
            path.len() > 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".wem")
        })
        .collect();
    let stride = (wems.len() / SOUND_SAMPLE).max(1);

    let outcomes: Vec<(String, Result<String, String>)> = wems
        .par_iter()
        .step_by(stride)
        .map(|entry| (entry.path().to_owned(), convert_sound(&archive, entry)))
        .collect();
    for entry in wems.iter().step_by(stride).take(SERVICE_SAMPLE) {
        let sound = Sound::from_archive(&archive, entry.path());
        assert!(
            matches!(
                sound,
                Ok(_)
                    | Err(ConversionError::Audio {
                        source: AudioError::PluginMedia,
                        ..
                    })
            ),
            "{}: {sound:?}",
            entry.path()
        );
    }

    let mut outcomes_by_kind: HashMap<String, usize> = HashMap::new();
    let mut failures = Vec::new();
    for (path, outcome) in &outcomes {
        match outcome {
            Ok(kind) => *outcomes_by_kind.entry(kind.clone()).or_default() += 1,
            Err(reason) => failures.push(format!("{path}: {reason}")),
        }
    }
    let mut summary: Vec<_> = outcomes_by_kind.iter().collect();
    summary.sort();
    writeln!(
        std::io::stderr(),
        "{} of {} WEMs sampled; per codec and outcome: {summary:?}",
        outcomes.len(),
        wems.len()
    )?;
    assert!(
        failures.is_empty(),
        "{} of {} WEMs failed: {:#?}",
        failures.len(),
        outcomes.len(),
        failures.iter().take(20).collect::<Vec<_>>()
    );
    Ok(())
}

fn convert_sound(archive: &Archive, entry: &Entry) -> Result<String, String> {
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .map_err(|error| error.to_string())?
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let wem = match Wem::parse(bytes.as_slice()) {
        Ok(wem) => wem,
        Err(AudioError::PluginMedia) => return Ok("plug-in media".to_owned()),
        Err(error) => return Err(error.to_string()),
    };
    let duration = wem
        .duration()
        .ok_or_else(|| format!("{} without a duration", wem.codec()))?;
    if duration.as_secs() > MAX_SOUND_SECONDS || wem.data().is_empty() {
        return Err(format!(
            "{}: implausible {duration:?} for {} data bytes",
            wem.codec(),
            wem.data().len()
        ));
    }
    let mut wav = Vec::new();
    match wem.write_wav(&mut wav) {
        Ok(report) => {
            assert_eq!(
                report.size,
                wav.len() as u64,
                "{}: reported size",
                entry.path()
            );
            Ok(format!("{}: WAV", wem.codec()))
        }
        Err(AudioError::UnsupportedCodec(codec)) if codec != Codec::Pcm => {
            Ok(format!("{codec}: no decoder yet"))
        }
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
