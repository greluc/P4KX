//! Every GFx and SWF movie of a real Archive is read and its embedded bitmaps decode; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;

use p4kx_extraction::gfx::{BitmapSource, EmbeddedFormat, MAX_BITMAP_PIXELS};
use p4kx_extraction::{Archive, Entry, Texture, UiMovie};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    movies: usize,
    definitions: usize,
    unknown_tags: usize,
    tag_failures: usize,
    embedded: usize,
    decoded: usize,
    largest: u64,
}

impl Tally {
    const fn add(self, other: Self) -> Self {
        Self {
            movies: self.movies + other.movies,
            definitions: self.definitions + other.definitions,
            unknown_tags: self.unknown_tags + other.unknown_tags,
            tag_failures: self.tag_failures + other.tag_failures,
            embedded: self.embedded + other.embedded,
            decoded: self.decoded + other.decoded,
            largest: if self.largest > other.largest {
                self.largest
            } else {
                other.largest
            },
        }
    }
}

fn read(archive: &Archive, entry: &Entry) -> Result<(Tally, Vec<String>), String> {
    let movie = UiMovie::from_archive(archive, entry.path()).map_err(|error| error.to_string())?;
    let assets = movie.movie().assets();
    let embedded: Vec<_> = assets
        .bitmaps
        .iter()
        .filter(|bitmap| match &bitmap.source {
            BitmapSource::Lossless(_) => true,
            BitmapSource::Jpeg(jpeg) => jpeg.format() != EmbeddedFormat::Gif,
            BitmapSource::External(_) | BitmapSource::SubImage(_) => false,
        })
        .collect();
    let largest = embedded
        .iter()
        .filter_map(|bitmap| assets.bitmap_size(bitmap).ok())
        .map(|(width, height)| u64::from(width) * u64::from(height))
        .max()
        .unwrap_or(0);
    let decoded = embedded
        .iter()
        .filter(|bitmap| assets.decode_bitmap(bitmap).is_ok())
        .count();
    let failures = assets
        .failures
        .iter()
        .map(|failure| {
            format!(
                "{} tag {} at {}: {}",
                movie.name(),
                failure.code,
                failure.offset,
                failure.error
            )
        })
        .collect();
    let tally = Tally {
        movies: 1,
        definitions: assets.shapes.len()
            + assets.morph_shapes.len()
            + assets.bitmaps.len()
            + assets.fonts.len()
            + assets.sprites.len(),
        unknown_tags: assets.unknown_tags.len(),
        tag_failures: assets.failures.len(),
        embedded: embedded.len(),
        decoded,
        largest,
    };
    Ok((tally, failures))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_real_ui_movie_is_read_without_failures() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let movies: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            entry.path().rsplit_once('.').is_some_and(|(_, extension)| {
                extension.eq_ignore_ascii_case("gfx") || extension.eq_ignore_ascii_case("swf")
            })
        })
        .collect();
    let results: Vec<Result<(Tally, Vec<String>), String>> = movies
        .par_iter()
        .map(|entry| read(&archive, entry))
        .collect();
    let errors: Vec<&String> = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect();
    let tally = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .fold(Tally::default(), |sum, (tally, _)| sum.add(*tally));
    let failures: Vec<&String> = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .flat_map(|(_, failures)| failures)
        .collect();
    assert!(
        movies.len() > 100,
        "the Archive holds UI movies: {}",
        movies.len()
    );
    assert!(errors.is_empty(), "unreadable movies: {errors:?}");
    assert!(failures.is_empty(), "unreadable tags: {failures:#?}");
    assert_eq!(tally.unknown_tags, 0, "unknown tags: {tally:?}");
    assert!(tally.definitions > 10_000, "definitions read: {tally:?}");
    assert_eq!(
        tally.decoded, tally.embedded,
        "undecodable bitmaps: {tally:?}"
    );
    assert!(
        tally.largest <= MAX_BITMAP_PIXELS,
        "the largest embedded bitmap fits the limit: {tally:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_menu_extracts_its_dds_backed_images() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let movie = UiMovie::from_archive(&archive, "Data\\UI\\Menus_Startmenu.gfx")?;
    let target = tempfile::tempdir()?;
    let extraction = movie.extract_assets(target.path())?;
    let dds_backed = movie
        .movie()
        .assets()
        .bitmaps
        .iter()
        .filter(|bitmap| matches!(&bitmap.source, BitmapSource::External(image) if image.format.name() == Some("dds")))
        .count();
    assert!(dds_backed > 0, "the menu refers to DDS textures");
    assert!(
        extraction.written.len() >= dds_backed,
        "written: {}, failed: {:?}",
        extraction.written.len(),
        extraction.failures
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_real_ui_texture_fits_the_bitmap_limit() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let textures: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path();
            path.get(..8)
                .is_some_and(|start| start.eq_ignore_ascii_case("Data\\UI\\"))
                && path
                    .rsplit_once('.')
                    .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("dds"))
        })
        .collect();
    let largest = textures
        .par_iter()
        .filter_map(|entry| Texture::from_archive(&archive, entry.path()).ok())
        .map(|texture| {
            let header = texture.header();
            (
                u64::from(header.width()) * u64::from(header.height()),
                texture.name().to_owned(),
            )
        })
        .max()
        .unwrap_or_default();
    assert!(textures.len() > 100, "UI textures: {}", textures.len());
    assert!(
        largest.0 <= MAX_BITMAP_PIXELS,
        "the largest UI texture fits the limit: {largest:?}"
    );
    Ok(())
}
