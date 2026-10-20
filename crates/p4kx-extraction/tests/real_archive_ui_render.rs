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
fn every_real_ui_movie_renders_its_first_frame() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
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
