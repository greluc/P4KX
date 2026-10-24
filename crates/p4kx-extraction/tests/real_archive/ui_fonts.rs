// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Fonts that UI movies import from other movies of a real Archive resolve and draw their texts
//! with glyphs; runs with `--ignored` when `P4KX_DATA_P4K` names one. The key is not needed: UI
//! movies are not encrypted.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_extraction::gfx::{Assets, Background, CanvasSize, RasterOptions};
use p4kx_extraction::{Archive, Entry, ImportIssue, UiMovie};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug, Default)]
struct Tally {
    movies: usize,
    text_fields: usize,
    own_font_fields: usize,
    imported_fields: usize,
    resolved_fields: usize,
    missing_font: usize,
    missing_glyph: usize,
    missing_by_movie: BTreeMap<String, usize>,
    unresolved: BTreeMap<String, usize>,
    errors: Vec<String>,
}

fn is_movie(entry: &Entry) -> bool {
    entry.path().rsplit_once('.').is_some_and(|(_, extension)| {
        extension.eq_ignore_ascii_case("gfx") || extension.eq_ignore_ascii_case("swf")
    })
}

fn tally_movie(archive: &Archive, entry: &Entry, tally: &mut Tally) -> TestResult {
    let movie = UiMovie::from_archive(archive, entry.path())?;
    let assets = movie.movie().assets();
    if assets.edit_texts.is_empty() && assets.static_texts.is_empty() {
        return Ok(());
    }
    let options = RasterOptions {
        size: CanvasSize::Width(256),
        background: Background::Transparent,
    };
    let render = movie.render(&options)?;
    tally.movies += 1;
    for skipped in &render.skipped {
        match skipped.reason.code() {
            "missing_font" => {
                tally.missing_font += 1;
                *tally
                    .missing_by_movie
                    .entry(format!("{} {}", movie.name(), skipped.reason))
                    .or_default() += 1;
            }
            "missing_glyph" | "missing_glyph_index" => tally.missing_glyph += 1,
            _ => {}
        }
    }
    let unresolved: Vec<&str> = render
        .imports
        .unresolved
        .iter()
        .map(|import| import.name.as_str())
        .collect();
    for unresolved in &render.imports.unresolved {
        *tally
            .unresolved
            .entry(format!(
                "{} {} {} {:?}",
                movie.name(),
                unresolved.url,
                unresolved.name,
                unresolved.issue
            ))
            .or_default() += 1;
    }
    count_fields(&assets, &unresolved, tally);
    Ok(())
}

fn count_fields(assets: &Assets<'_>, unresolved: &[&str], tally: &mut Tally) {
    let font_ids = assets.edit_texts.iter().map(|text| text.font_id).chain(
        assets
            .static_texts
            .iter()
            .flat_map(|text| text.records.iter().map(|record| record.font_id)),
    );
    for font_id in font_ids {
        tally.text_fields += 1;
        let Some(id) = font_id else { continue };
        if assets.fonts.iter().any(|font| font.id == id) {
            tally.own_font_fields += 1;
        } else if let Some((name, _)) = assets.import_of(id) {
            tally.imported_fields += 1;
            if !unresolved.contains(&name) {
                tally.resolved_fields += 1;
            }
        }
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stderr,
    reason = "the counts of the real Archive are the output"
)]
fn the_tags_of_real_ui_movies_that_are_not_read_are_counted_by_code() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let mut unread: BTreeMap<u16, u32> = BTreeMap::new();
    let (mut movies, mut failures, mut unknown, mut labels, mut lists) = (0_usize, 0, 0, 0, 0);
    let mut compacted = 0_usize;
    for entry in archive.entries().iter().filter(|entry| is_movie(entry)) {
        let movie = UiMovie::from_archive(&archive, entry.path())?;
        let assets = movie.movie().assets();
        movies += 1;
        failures += assets.failures.len();
        unknown += assets.unknown_tags.len();
        labels += assets.timeline.labels.len();
        lists += assets.timeline.frame_lists.len();
        compacted += assets
            .tag_counts
            .iter()
            .filter(|count| count.code == 1005)
            .map(|count| usize::try_from(count.count).unwrap_or(0))
            .sum::<usize>();
        for count in &assets.unread_tags {
            *unread.entry(count.code).or_default() += count.count;
        }
    }
    let named: Vec<String> = unread
        .iter()
        .map(|(&code, count)| {
            format!(
                "{code} {} x{count}",
                p4kx_extraction::gfx::tag_name(code).unwrap_or("?")
            )
        })
        .collect();
    eprintln!(
        "movies {movies}, labels {labels}, frame lists {lists}, compacted fonts {compacted}; \
         unread tags: {named:#?}"
    );
    assert!(movies > 100, "UI movies: {movies}");
    assert_eq!((failures, unknown), (0, 0), "every tag is read or known");
    for code in [11, 33, 37, 57, 71, 76, 78] {
        assert!(!unread.contains_key(&code), "tag {code} is read");
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stderr,
    reason = "the counts of the real Archive are the output"
)]
fn imported_fonts_of_real_ui_movies_resolve() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let mut tally = Tally::default();
    for entry in archive.entries().iter().filter(|entry| is_movie(entry)) {
        if let Err(error) = tally_movie(&archive, entry, &mut tally) {
            tally.errors.push(format!("{}: {error}", entry.path()));
        }
    }
    eprintln!("{tally:#?}");
    assert!(tally.errors.is_empty(), "{:#?}", tally.errors);
    assert!(tally.movies > 100, "movies with text: {}", tally.movies);
    assert!(
        tally.imported_fields > 0,
        "movies import fonts: {}",
        tally.imported_fields
    );
    assert!(
        tally.resolved_fields * 10 >= tally.imported_fields * 9,
        "at least 90 % of the imported fonts resolve: {tally:#?}"
    );
    assert!(
        tally.missing_font < 100,
        "imported fonts no longer draw as boxes: {} placements still miss their font",
        tally.missing_font
    );
    assert!(
        !tally
            .unresolved
            .keys()
            .any(|key| key.contains(&format!("{:?}", ImportIssue::TooDeep))),
        "no import is too deep: {tally:#?}"
    );
    Ok(())
}
