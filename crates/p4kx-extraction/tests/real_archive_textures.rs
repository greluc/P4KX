//! Extraction with texture Conversion on a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

use p4kx_extraction::{Archive, ConflictPolicy, Entry, EntryFailure, ExtractOptions, extract};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const TEXTURE_SAMPLE: usize = 400;

fn written(target: &Path, entry_path: &str, extension: Option<&str>) -> bool {
    let mut path = target.join(entry_path.replace('\\', "/"));
    if let Some(extension) = extension {
        path.set_extension(extension);
    }
    path.is_file()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn converts_a_sample_of_real_textures_once_each() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let mut seen = HashSet::new();
    let headers: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".dds"))
        .filter(|entry| seen.insert(entry.path().replace('/', "\\").to_lowercase()))
        .collect();
    let stride = (headers.len() / TEXTURE_SAMPLE).max(1);
    let sample: Vec<&Entry> = headers.into_iter().step_by(stride).collect();
    let target = tempfile::tempdir()?;
    let options = ExtractOptions {
        convert: true,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &sample, target.path(), &options);

    assert!(
        report.failures().is_empty(),
        "failed: {:?}",
        report.failures()
    );
    assert_eq!(report.extracted(), sample.len(), "every header is written");
    let unconverted: HashSet<&str> = report
        .unconverted()
        .iter()
        .map(EntryFailure::path)
        .collect();
    assert!(
        unconverted.len() < sample.len(),
        "most textures convert: {:?}",
        report.unconverted()
    );
    for entry in &sample {
        assert!(
            written(target.path(), entry.path(), None),
            "{} has a file",
            entry.path()
        );
        if !unconverted.contains(entry.path()) {
            assert!(
                written(target.path(), entry.path(), Some("png")),
                "{} has a PNG",
                entry.path()
            );
        }
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
        "every output is kept: {:?}",
        again.failures()
    );
    Ok(())
}
