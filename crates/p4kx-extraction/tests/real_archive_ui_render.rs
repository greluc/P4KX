//! The first frame of every GFx and SWF movie of a real Archive renders without errors or
//! rasteriser failures; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions};
use p4kx_extraction::{Archive, Entry, UiMovie};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug, Default)]
struct Tally {
    movies: usize,
    drawn: usize,
    bitmap_failures: usize,
    skipped: BTreeMap<&'static str, usize>,
    errors: Vec<String>,
}

fn render(archive: &Archive, entry: &Entry, tally: &mut Tally) {
    let options = RasterOptions {
        size: CanvasSize::Width(256),
        background: Background::Transparent,
    };
    let rendered =
        UiMovie::from_archive(archive, entry.path()).and_then(|movie| movie.render(&options));
    tally.movies += 1;
    match rendered {
        Ok(render) => {
            if render
                .image
                .pixels()
                .chunks(4)
                .any(|pixel| pixel.get(3) != Some(&0))
            {
                tally.drawn += 1;
            }
            tally.bitmap_failures += render.bitmap_failures.len();
            for skipped in &render.skipped {
                *tally.skipped.entry(skipped.reason.code()).or_default() += 1;
            }
        }
        Err(error) => tally.errors.push(format!("{}: {error}", entry.path())),
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_real_ui_movie_writes_a_well_formed_svg_that_skips_what_the_raster_skips() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("movie.svg");
    let options = RasterOptions {
        size: CanvasSize::Width(256),
        background: Background::Transparent,
    };
    let mut movies = 0_usize;
    let mut largest = 0_u64;
    for entry in archive.entries().iter().filter(|entry| {
        entry.path().rsplit_once('.').is_some_and(|(_, extension)| {
            extension.eq_ignore_ascii_case("gfx") || extension.eq_ignore_ascii_case("swf")
        })
    }) {
        let movie = UiMovie::from_archive(&archive, entry.path())?;
        let svg = movie.render_svg_to_file(&options, &out)?;
        let raster = movie.render(&options)?;
        assert_eq!(
            svg.skipped,
            raster.skipped,
            "{}: the same elements are left out",
            entry.path()
        );
        let text = std::fs::read_to_string(&out)?;
        roxmltree::Document::parse(&text)
            .map_err(|error| format!("{}: not well-formed: {error}", entry.path()))?;
        largest = largest.max(u64::try_from(text.len())?);
        movies += 1;
    }
    assert!(movies > 100, "UI movies: {movies}");
    assert!(largest < 1 << 30, "the largest SVG has {largest} bytes");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_real_ui_movie_renders_its_first_frame() -> TestResult {
    let archive =
        Archive::open_with_key(real_archive_path()?, p4kx_testkit::real_archive_key().ok())?;
    let movies: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            entry.path().rsplit_once('.').is_some_and(|(_, extension)| {
                extension.eq_ignore_ascii_case("gfx") || extension.eq_ignore_ascii_case("swf")
            })
        })
        .collect();
    let mut tally = Tally::default();
    for entry in &movies {
        render(&archive, entry, &mut tally);
    }
    assert!(tally.movies > 100, "UI movies: {tally:?}");
    assert!(
        tally.errors.is_empty(),
        "movies that do not render: {tally:#?}"
    );
    assert_eq!(
        tally.skipped.get("raster"),
        None,
        "rasteriser failures: {tally:?}"
    );
    for limit in [
        "fill_budget",
        "too_many_records",
        "too_many_paths",
        "too_many_placements",
        "too_many_masks",
        "mask_memory",
        "bitmap_memory",
        "too_many_glyphs",
        "text_truncated",
    ] {
        assert_eq!(
            tally.skipped.get(limit),
            None,
            "real movies stay within the {limit} limit: {tally:?}"
        );
    }
    assert!(
        tally.drawn * 2 > tally.movies,
        "most movies draw something: {tally:?}"
    );
    Ok(())
}
