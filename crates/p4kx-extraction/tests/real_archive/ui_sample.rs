// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A deterministic sample of the GFx and SWF movies of a real Archive, every Nth by sorted path,
//! parses and renders without a panic or an error.

use std::error::Error;
use std::time::Instant;

use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions};
use p4kx_extraction::{Archive, Entry, UiMovie};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE_SIZE: usize = 300;
const MIN_SAMPLE: usize = 100;

fn is_movie(entry: &Entry) -> bool {
    entry.path().rsplit_once('.').is_some_and(|(_, extension)| {
        extension.eq_ignore_ascii_case("gfx") || extension.eq_ignore_ascii_case("swf")
    })
}

fn render(archive: &Archive, entry: &Entry) -> Result<bool, String> {
    let options = RasterOptions {
        size: CanvasSize::Width(256),
        background: Background::Transparent,
    };
    let movie = UiMovie::from_archive(archive, entry.path())
        .map_err(|error| format!("{}: {error}", entry.path()))?;
    let render = movie
        .render(&options)
        .map_err(|error| format!("{}: {error}", entry.path()))?;
    Ok(render
        .image
        .pixels()
        .chunks(4)
        .any(|pixel| pixel.get(3) != Some(&0)))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the summary of the run is for the reader of --nocapture"
)]
fn a_sample_of_real_ui_movies_parses_and_renders() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let mut movies: Vec<&Entry> = archive.entries().iter().filter(|e| is_movie(e)).collect();
    movies.sort_by(|a, b| a.path().cmp(b.path()));
    let stride = (movies.len() / SAMPLE_SIZE).max(1);
    let sample: Vec<&Entry> = movies.iter().copied().step_by(stride).collect();
    let started = Instant::now();
    let results: Vec<Result<bool, String>> = sample
        .par_iter()
        .map(|entry| render(&archive, entry))
        .collect();
    let elapsed = started.elapsed();
    let errors: Vec<&String> = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect();
    let drawn = results.iter().filter(|result| **result == Ok(true)).count();
    println!(
        "{} of {} movies sampled (every {stride}th): {} rendered, {drawn} with visible pixels, {} errors, {} ms",
        sample.len(),
        movies.len(),
        results.len() - errors.len(),
        errors.len(),
        elapsed.as_millis()
    );
    assert!(sample.len() >= MIN_SAMPLE, "sample size: {}", sample.len());
    assert!(errors.is_empty(), "movies that do not render: {errors:#?}");
    assert!(drawn > 0, "some sampled movie draws pixels");
    Ok(())
}
