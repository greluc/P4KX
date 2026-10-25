// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The order of a split cubemap's faces in the header Entry's tail, checked against the mip
//! chain: the mean colour of each face of a mip must follow that of the same face one mip up,
//! which holds only if the tail is read in the right order.

use std::io::Read;

use p4kx_extraction::{Archive, DdsHeader, Entry, RgbaImage, Texture};
use p4kx_testkit::real_archive_path;
use p4kx_texture::MAX_HEADER_LEN;
use rayon::prelude::*;

use crate::{TestResult, report};

const FACES: u32 = 6;
const MIN_FACE_SPREAD: u64 = 400;

fn mean(image: &RgbaImage) -> [u64; 3] {
    let (pixels, _) = image.pixels().as_chunks::<4>();
    let count = pixels.len().max(1) as u64;
    let mut sum = [0_u64; 3];
    for pixel in pixels {
        for (total, &value) in sum.iter_mut().zip(pixel) {
            *total += u64::from(value) * 16;
        }
    }
    sum.map(|total| total / count)
}

fn distance(a: [u64; 3], b: [u64; 3]) -> u64 {
    a.iter().zip(b).map(|(&x, y)| x.abs_diff(y)).sum()
}

#[derive(Default)]
struct Verdict {
    pairs: usize,
    separable: usize,
    in_order: usize,
    worst: u64,
}

fn check(archive: &Archive, entry: &Entry) -> Result<Option<Verdict>, String> {
    let mut start = Vec::new();
    archive
        .open_entry(entry)
        .map_err(|error| error.to_string())?
        .take(MAX_HEADER_LEN as u64)
        .read_to_end(&mut start)
        .map_err(|error| error.to_string())?;
    let Ok(header) = DdsHeader::parse(&start) else {
        return Ok(None);
    };
    if header.faces() != FACES || header.array_size() != 1 {
        return Ok(None);
    }
    let parts = header
        .part_count(entry.uncompressed_size())
        .map_err(|error| error.to_string())?;
    let in_tail = header.mip_count().saturating_sub(parts);
    if parts == 0 || in_tail < 2 {
        return Ok(None);
    }
    let texture =
        Texture::from_archive(archive, entry.path()).map_err(|error| error.to_string())?;
    let mut verdict = Verdict::default();
    for level in parts - 1..header.mip_count() - 1 {
        let faces = |level| -> Result<Vec<[u64; 3]>, String> {
            let images = texture
                .decode_surfaces(level)
                .map_err(|error| format!("{}: {error}", entry.path()))?;
            Ok(images.iter().map(mean).collect())
        };
        let (upper, lower) = (faces(level)?, faces(level + 1)?);
        verdict.pairs += 1;
        let own: u64 = upper
            .iter()
            .zip(&lower)
            .map(|(&a, &b)| distance(a, b))
            .sum();
        let shifted: u64 = upper
            .iter()
            .zip(lower.iter().cycle().skip(1))
            .map(|(&a, &b)| distance(a, b))
            .sum();
        verdict.worst = verdict.worst.max(own);
        if shifted.saturating_sub(own) >= MIN_FACE_SPREAD {
            verdict.separable += 1;
            if own < shifted {
                verdict.in_order += 1;
            }
        }
    }
    Ok(Some(verdict))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_tail_mips_of_real_split_cubemaps_follow_the_faces_of_the_mips_above() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entries: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            Texture::is_header_path(entry.path())
                && !entry.path().to_ascii_lowercase().ends_with(".dds.a")
        })
        .collect();
    let verdicts: Vec<Result<Option<Verdict>, String>> = entries
        .par_iter()
        .map(|entry| check(&archive, entry))
        .collect();
    let mut total = Verdict::default();
    let mut cubemaps = 0;
    let mut failures = Vec::new();
    for verdict in verdicts {
        match verdict {
            Ok(Some(verdict)) => {
                cubemaps += 1;
                total.pairs += verdict.pairs;
                total.separable += verdict.separable;
                total.in_order += verdict.in_order;
                total.worst = total.worst.max(verdict.worst);
            }
            Ok(None) => {}
            Err(error) => failures.push(error),
        }
    }
    report(&format!(
        "{cubemaps} split cubemaps with two or more mips in the header tail, {} mip pairs, {} \
         told face order apart, {} of them in order, worst distance {}",
        total.pairs, total.separable, total.in_order, total.worst
    ));
    assert!(failures.is_empty(), "failures: {failures:?}");
    assert!(total.separable > 0, "some pairs tell the face order apart");
    assert_eq!(
        total.in_order, total.separable,
        "every pair follows its face"
    );
    Ok(())
}
