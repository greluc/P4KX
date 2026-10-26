// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The header-only listing of the animations of a model against reading them in full.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use p4kx_archive::Archive;
use p4kx_extraction::model::{Animation, AnimationHeader, Dba, DbaHeaders, LodSelection};
use p4kx_extraction::{
    AnimationChoice, AnimationListing, AnimationSet, ChunkSource, ModelInfo, ModelSource, PathIndex,
};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

use super::TestResult;

const MODELS: usize = 5;
const RUNS: usize = 3;
const DURATION_TOLERANCE: f32 = 1e-4;

fn has_extension(path: &str, extensions: &[&str]) -> bool {
    Path::new(path).extension().is_some_and(|found| {
        extensions
            .iter()
            .any(|wanted| found.eq_ignore_ascii_case(wanted))
    })
}

fn folder_of(path: &str) -> String {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    path.rsplit_once('/')
        .map_or_else(String::new, |(folder, _)| folder.to_owned())
}

fn crowded_models(archive: &Archive) -> Vec<(String, usize, u64)> {
    let mut counts: HashMap<String, (usize, u64)> = HashMap::new();
    for entry in archive.entries() {
        if has_extension(entry.path(), &["caf", "dba"]) {
            let slot = counts.entry(folder_of(entry.path())).or_default();
            slot.0 += 1;
            slot.1 += entry.uncompressed_size();
        }
    }
    let mut models: HashMap<String, String> = HashMap::new();
    for entry in archive.entries() {
        let path = entry.path();
        if has_extension(path, &["cga", "cgf", "skin", "chr"]) {
            models
                .entry(folder_of(path))
                .or_insert_with(|| path.to_owned());
        }
    }
    let mut found: Vec<(String, usize, u64)> = Vec::new();
    for (folder, (files, bytes)) in &counts {
        let model = models.get(folder).or_else(|| {
            folder
                .strip_prefix("data/animations/")
                .and_then(|rest| models.get(&format!("data/{rest}")))
        });
        if let Some(model) = model {
            found.push((model.clone(), *files, *bytes));
        }
    }
    found.sort_by_key(|(_, _, bytes)| std::cmp::Reverse(*bytes));
    found.truncate(MODELS);
    found
}

fn milliseconds(started: Instant) -> u128 {
    started.elapsed().as_millis()
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_listing_costs_less_than_reading_the_animations_of_crowded_models() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let index = PathIndex::new(&archive);
    let mut models = crowded_models(&archive);
    models.push((super::SKELETON.to_owned(), 0, 0));
    for (model, files, bytes) in models {
        for run in 0..RUNS {
            let started = Instant::now();
            let set = AnimationSet::from_archive(&index, &model, &AnimationChoice::All, &[])?;
            let full = milliseconds(started);
            let started = Instant::now();
            let listing = AnimationListing::from_archive(&index, &model, &[])?;
            let listed = milliseconds(started);
            println!(
                "{model}: {files} files, {bytes} bytes, {} animations; full read {full} ms, listing {listed} ms (run {run})",
                set.animations.len(),
            );
            assert_eq!(listing.animations.len(), set.animations.len(), "count");
            assert_eq!(listing.origins, set.origins, "origins");
            assert_eq!(listing.unread, set.unread, "unread");
        }
    }
    let extra = [super::GETUP.to_owned()];
    for run in 0..RUNS {
        let started = Instant::now();
        let set =
            AnimationSet::from_archive(&index, super::SKELETON, &AnimationChoice::All, &extra)?;
        let full = milliseconds(started);
        let started = Instant::now();
        let listing = AnimationListing::from_archive(&index, super::SKELETON, &extra)?;
        let listed = milliseconds(started);
        println!(
            "--anim-from getup.dba: {} animations; full read {full} ms, listing {listed} ms (run {run})",
            set.animations.len(),
        );
        assert_eq!(listing.animations.len(), set.animations.len(), "count");
    }
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_phases_of_model_info_for_a_crowded_model() -> TestResult {
    let started = Instant::now();
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    println!("open the Archive: {} ms", milliseconds(started));
    let Some((model, files, _)) = crowded_models(&archive).into_iter().nth(1) else {
        return Err("no crowded model".into());
    };
    let started = Instant::now();
    let index = PathIndex::new(&archive);
    println!("index the paths: {} ms", milliseconds(started));
    let started = Instant::now();
    let source = ModelSource::from_index_with(&index, &model, LodSelection::All)?;
    println!("read {model}: {} ms", milliseconds(started));
    let started = Instant::now();
    let set = AnimationSet::from_archive(&index, &model, &AnimationChoice::All, &[])?;
    println!(
        "{files} animation files, full read: {} ms",
        milliseconds(started)
    );
    let started = Instant::now();
    let listing = AnimationListing::from_archive(&index, &model, &[])?;
    println!("same, listing: {} ms", milliseconds(started));
    let started = Instant::now();
    let info = ModelInfo::of_listing(&source, &listing);
    println!(
        "describe {} animations: {} ms ({} in the full read)",
        info.animations.len(),
        milliseconds(started),
        set.animations.len()
    );
    Ok(())
}

fn differences(headers: &[AnimationHeader], full: &[Animation], path: &str) -> Vec<String> {
    let mut differing = Vec::new();
    for (header, full) in headers.iter().zip(full) {
        let same = header.name == full.name
            && header.end_frame == full.end_frame
            && header.keys == full.key_count()
            && header.additive == full.additive
            && header.controller_ids.len() == full.tracks.len()
            && header
                .controller_ids
                .iter()
                .zip(&full.tracks)
                .all(|(id, track)| *id == track.controller_id)
            && (header.duration_seconds() - full.duration_seconds()).abs() < DURATION_TOLERANCE;
        if !same {
            differing.push(format!("{path}: {}", full.name));
        }
    }
    differing
}

fn compare(index: &PathIndex<'_>, path: &str) -> Result<(usize, Vec<String>), String> {
    let source = ChunkSource::from_index(index, path).map_err(|error| error.to_string())?;
    if has_extension(path, &["caf"]) {
        let header = AnimationHeader::read_caf(path, source.file());
        return match (header, Animation::read_caf(path, source.file())) {
            (Ok(header), Ok(full)) => Ok((1, differences(&[header], &[full], path))),
            (_, Err(_)) => Ok((0, Vec::new())),
            (Err(error), Ok(_)) => Err(format!("{path}: {error}")),
        };
    }
    let headers = DbaHeaders::read(path, source.file()).map_err(|error| error.to_string())?;
    let full = Dba::read(path, source.file()).map_err(|error| error.to_string())?;
    let kept: Vec<AnimationHeader> = headers
        .animations
        .into_iter()
        .filter(|header| full.animations.iter().any(|read| read.path == header.path))
        .collect();
    Ok((
        full.animations.len(),
        differences(&kept, &full.animations, path),
    ))
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_header_of_the_archive_agrees_with_the_decoded_animation() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let index = PathIndex::new(&archive);
    let paths: Vec<&str> = archive
        .entries()
        .iter()
        .map(p4kx_archive::Entry::path)
        .filter(|path| has_extension(path, &["caf", "dba"]))
        .collect();
    let started = Instant::now();
    let (animations, differing) = paths
        .par_iter()
        .map(|path| compare(&index, path))
        .try_reduce(
            || (0, Vec::new()),
            |(count, mut left), (more, right)| {
                left.extend(right);
                Ok((count + more, left))
            },
        )?;
    println!(
        "{animations} animations compared in {} ms; {} differ",
        milliseconds(started),
        differing.len()
    );
    assert!(
        differing.is_empty(),
        "differ: {:?}",
        differing.get(..10.min(differing.len()))
    );
    Ok(())
}
