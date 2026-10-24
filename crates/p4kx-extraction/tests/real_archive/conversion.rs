// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Extraction with `CryXml` Conversion on a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::collections::HashSet;
use std::error::Error;

use p4kx_extraction::{Archive, ConflictPolicy, Entry, ExtractOptions, extract};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const MAX_SIZE: u64 = 4 << 20;
const CONVERSION_SAMPLE: usize = 2000;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn extracts_a_sample_of_real_xml_entries_with_conversion() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
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
