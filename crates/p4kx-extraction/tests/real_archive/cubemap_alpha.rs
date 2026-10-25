// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Split cubemaps and alpha tails of the real Archive.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{Read, Write};

use p4kx_extraction::{Archive, DdsHeader, Entry, Texture};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::TestResult;

type CheckResult = Result<String, Box<dyn Error + Send + Sync>>;

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
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
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

fn check_cubemap(archive: &Archive, texture: &Texture<'_>) -> CheckResult {
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
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let tails: Vec<&Entry> = archive
        .entries()
        .par_iter()
        .filter(|entry| ends_with(entry, ".dds.a"))
        .collect();
    assert!(!tails.is_empty(), "the Archive has alpha tails");
    let stride = (tails.len() / ALPHA_SAMPLE).max(1);
    let several: Vec<&Entry> = tails
        .par_iter()
        .copied()
        .filter(|tail| holds_several_images(&archive, tail))
        .collect();

    let outcomes: Vec<Result<String, String>> = tails
        .par_iter()
        .step_by(stride)
        .chain(several.par_iter())
        .map(|tail| check_alpha_tail(&archive, tail).map_err(|error| error.to_string()))
        .collect();

    let mut summary: BTreeMap<&str, usize> = BTreeMap::new();
    for outcome in outcomes.iter().flatten() {
        *summary.entry(outcome.as_str()).or_default() += 1;
    }
    writeln!(
        std::io::stderr(),
        "{} alpha tails, {} of array or volume textures, {} checked: {summary:#?}",
        tails.len(),
        several.len(),
        outcomes.len()
    )?;
    let failures: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().err())
        .collect();
    assert!(failures.is_empty(), "alpha tails failed: {failures:#?}");
    assert!(
        summary.keys().any(|kind| kind.contains("BC4_UNORM")),
        "real BC4 tails are decoded: {summary:?}"
    );
    Ok(())
}

fn holds_several_images(archive: &Archive, tail: &Entry) -> bool {
    let colour = tail.path().get(..tail.path().len() - 2).unwrap_or_default();
    Texture::from_archive(archive, colour).is_ok_and(|texture| {
        let header = texture.header();
        header.array_size() > 1 || header.depth() > 1
    })
}

/// Checks that the tail of `tail` is found, merges on its own, and decodes into the alpha channel
/// of the colour texture's largest mip, every image of a cubemap, array or volume included, every
/// pixel's alpha being the tail's decoded value: its alpha for an `A8_UNORM` tail, else its red.
fn check_alpha_tail(archive: &Archive, tail: &Entry) -> CheckResult {
    let colour_path = tail
        .path()
        .get(..tail.path().len() - 2)
        .ok_or("a tail path ends in .a")?;
    let texture = Texture::from_archive(archive, colour_path)?;
    let found = texture
        .alpha_tail()
        .ok_or_else(|| format!("{colour_path}: the alpha tail is not found"))??;
    let (colour, alpha) = (texture.header(), found.header());
    if (alpha.width(), alpha.height()) != (colour.width(), colour.height()) {
        return Err(format!("{colour_path}: the alpha tail has another size").into());
    }
    let mut merged = Vec::new();
    let report = Texture::from_archive(archive, tail.path())?.merge_into(&mut merged)?;
    if report.size != merged.len() as u64 {
        return Err(format!("{}: merged size", tail.path()).into());
    }
    let channel = if alpha.format().to_string() == "A8_UNORM" {
        3
    } else {
        0
    };
    let images = texture
        .decode_surfaces(0)
        .map_err(|error| format!("{colour_path}: {error}"))?;
    let values = found
        .decode_surfaces(0)
        .map_err(|error| format!("{}: {error}", tail.path()))?;
    if images.len() != values.len() {
        return Err(format!("{colour_path}: the tail has another number of images").into());
    }
    for (image, value) in images.iter().zip(&values) {
        let (pixels, _) = image.pixels().as_chunks::<4>();
        let (tail_pixels, _) = value.pixels().as_chunks::<4>();
        if pixels.len() != tail_pixels.len()
            || pixels
                .iter()
                .zip(tail_pixels)
                .any(|(pixel, value)| Some(&pixel[3]) != value.get(channel))
        {
            return Err(format!("{colour_path}: the alpha is not the tail's value").into());
        }
    }
    Ok(format!(
        "{} + {}: decoded into alpha, {} images",
        colour.format(),
        alpha.format(),
        images.len()
    ))
}
