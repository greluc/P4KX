// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Every floating-point texture of the real Archive: detected as high range, decoded without NaN,
//! exported as OpenEXR with one file per image, and without an alpha tail that the export would
//! drop.

use std::collections::BTreeMap;
use std::io::Read;

use p4kx_extraction::{Archive, DdsHeader, Entry, Texture};
use p4kx_testkit::real_archive_path;
use p4kx_texture::MAX_HEADER_LEN;
use rayon::prelude::*;

use crate::{TestResult, report};

const EXR_SIDE: u32 = 512;

fn is_float(entry: &Entry, archive: &Archive) -> Option<DdsHeader> {
    if !Texture::is_header_path(entry.path())
        || entry.path().to_ascii_lowercase().ends_with(".dds.a")
    {
        return None;
    }
    let mut start = Vec::new();
    archive
        .open_entry(entry)
        .ok()?
        .take(MAX_HEADER_LEN as u64)
        .read_to_end(&mut start)
        .ok()?;
    let header = DdsHeader::parse(&start).ok()?;
    header
        .format()
        .to_string()
        .ends_with("_FLOAT")
        .then_some(header)
}

fn check(archive: &Archive, path: &str) -> Result<String, String> {
    let texture = Texture::from_archive(archive, path).map_err(|error| error.to_string())?;
    let header = *texture.header();
    let format = header.format();
    if !format.is_high_range() {
        return Err(format!("{path}: {format} is not high range"));
    }
    if texture.has_alpha_tail() {
        return Err(format!("{path}: an alpha tail the EXR export would drop"));
    }
    let level = (0..header.mip_count())
        .find(|&level| (header.width() >> level).max(header.height() >> level) <= EXR_SIDE)
        .unwrap_or_else(|| header.mip_count() - 1);
    let images = texture
        .decode_surfaces_hdr(level)
        .map_err(|error| format!("{path}: {error}"))?;
    if images.len() != usize::try_from(header.images(level)).unwrap_or(0) {
        return Err(format!("{path}: {} images decoded", images.len()));
    }
    if let Some(value) = images
        .iter()
        .flat_map(p4kx_extraction::HdrImage::pixels)
        .find(|value| value.is_nan())
    {
        return Err(format!("{path}: value {value}"));
    }
    let folder = tempfile::tempdir().map_err(|error| error.to_string())?;
    let report = texture
        .exr_to_file(&folder.path().join("image.exr"), level)
        .map_err(|error| format!("{path}: {error}"))?;
    if report.files.len() != images.len() || report.files.iter().any(|file| !file.is_file()) {
        return Err(format!("{path}: {} EXR files written", report.files.len()));
    }
    Ok(format.to_string())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn real_floating_point_textures_export_as_openexr() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let floats: Vec<&Entry> = archive
        .entries()
        .par_iter()
        .filter(|entry| is_float(entry, &archive).is_some())
        .collect();
    assert!(
        !floats.is_empty(),
        "the Archive has floating-point textures"
    );
    let outcomes: Vec<Result<String, String>> = floats
        .par_iter()
        .map(|entry| check(&archive, entry.path()))
        .collect();
    let mut formats: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok(format) => *formats.entry(format).or_default() += 1,
            Err(error) => failures.push(error),
        }
    }
    report(&format!(
        "{} floating-point textures exported as OpenEXR: {formats:?}",
        floats.len()
    ));
    assert!(failures.is_empty(), "failures: {failures:#?}");
    Ok(())
}
