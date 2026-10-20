//! Texture Conversion during an Extraction of the real Archive.

use std::collections::HashSet;
use std::path::Path;

use p4kx_extraction::{
    Archive, ConflictPolicy, Entry, EntryFailure, ExtractOptions, ExtractionReport, extract,
};
use p4kx_testkit::real_archive_path;

use crate::TestResult;

const TEXTURE_SAMPLE: usize = 400;
const TAILED_SAMPLE: usize = 60;

fn has_suffix(path: &str, suffix: &str) -> bool {
    path.len()
        .checked_sub(suffix.len())
        .and_then(|start| path.get(start..))
        .is_some_and(|end| end.eq_ignore_ascii_case(suffix))
}

fn output(target: &Path, entry_path: &str, extension: Option<&str>) -> bool {
    let mut path = target.join(entry_path.replace('\\', "/"));
    if let Some(extension) = extension {
        path.set_extension(extension);
    }
    path.is_file()
}

fn sample<'a>(entries: impl Iterator<Item = &'a Entry>, size: usize) -> Vec<&'a Entry> {
    let mut seen = HashSet::new();
    let unique: Vec<&Entry> = entries
        .filter(|entry| seen.insert(entry.path().replace('/', "\\").to_lowercase()))
        .collect();
    let stride = (unique.len() / size).max(1);
    unique.into_iter().step_by(stride).collect()
}

fn convert(archive: &Archive, sample: &[&Entry], target: &Path) -> ExtractionReport {
    let options = ExtractOptions {
        convert: true,
        ..ExtractOptions::default()
    };
    extract(archive, sample, target, &options)
}

fn has_image(target: &Path, header: &str) -> bool {
    ["png", "exr"].into_iter().any(|extension| {
        output(target, header, Some(extension))
            || output(
                target,
                &header.replace(".dds", "_posx.dds"),
                Some(extension),
            )
    })
}

/// Checks that no Entry failed and most `headers` converted; returns those that did.
fn converted<'a>(report: &ExtractionReport, headers: &[&'a Entry]) -> Vec<&'a Entry> {
    assert!(
        report.failures().is_empty(),
        "failed: {:?}",
        report.failures()
    );
    let unconverted: HashSet<&str> = report
        .unconverted()
        .iter()
        .map(EntryFailure::path)
        .collect();
    let converted: Vec<&Entry> = headers
        .iter()
        .copied()
        .filter(|header| !unconverted.contains(header.path()))
        .collect();
    assert!(
        converted.len() > headers.len() / 2,
        "most textures convert: {:?}",
        report.unconverted()
    );
    converted
}

/// Returns a sample of the headers that have an alpha tail, and every Entry of their textures.
fn tailed_textures(archive: &Archive) -> (Vec<&Entry>, Vec<&Entry>) {
    let tails: HashSet<String> = archive
        .entries()
        .iter()
        .filter(|entry| has_suffix(entry.path(), ".dds.a"))
        .filter_map(|entry| entry.path().get(..entry.path().len() - 2))
        .map(|header| header.replace('/', "\\").to_lowercase())
        .collect();
    let headers = sample(
        archive.entries().iter().filter(|entry| {
            has_suffix(entry.path(), ".dds")
                && tails.contains(&entry.path().replace('/', "\\").to_lowercase())
        }),
        TAILED_SAMPLE,
    );
    let members = archive
        .entries()
        .iter()
        .filter(|entry| {
            headers
                .iter()
                .any(|header| entry.path().starts_with(header.path()))
        })
        .collect();
    (headers, members)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn converts_a_sample_of_real_textures_once_each() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let headers = sample(
        archive
            .entries()
            .iter()
            .filter(|entry| has_suffix(entry.path(), ".dds")),
        TEXTURE_SAMPLE,
    );
    let target = tempfile::tempdir()?;

    let report = convert(&archive, &headers, target.path());

    assert_eq!(report.extracted(), headers.len(), "every header is written");
    for header in converted(&report, &headers) {
        assert!(
            has_image(target.path(), header.path()),
            "{} has a PNG or OpenEXR",
            header.path()
        );
    }
    let options = ExtractOptions {
        policy: ConflictPolicy::IfChanged,
        convert: true,
        ..ExtractOptions::default()
    };
    let again = extract(&archive, &headers, target.path(), &options);
    assert_eq!(
        again.skipped().len(),
        headers.len(),
        "every output is kept: {:?}",
        again.failures()
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn converts_real_textures_with_alpha_tails_without_writing_the_tails() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let (headers, members) = tailed_textures(&archive);
    assert!(headers.len() >= 10, "the Archive has textures with tails");
    let target = tempfile::tempdir()?;

    let report = convert(&archive, &members, target.path());

    assert_eq!(
        report.extracted(),
        members.len(),
        "every Entry ends written"
    );
    for header in converted(&report, &headers) {
        let tail = format!("{}.a", header.path());
        assert!(
            !output(target.path(), &tail, None),
            "{tail} is not written raw"
        );
        assert!(
            has_image(target.path(), header.path()),
            "{} has a PNG or OpenEXR",
            header.path()
        );
    }
    Ok(())
}
