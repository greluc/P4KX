// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Imports of one name from different movies, and the movie's own export of that name.

use p4kx_extraction::{ImportIssue, UiMovie};
use p4kx_testkit::{TestEntry, exports_body, font3_body};

use crate::ui_import_support::{
    CLIENT, TestResult, archive, client_of, font_movie_with, glyph, green_pixels, options, stage,
};

const NARROW: i16 = 5_000;
const WIDE: i16 = 20_000;
const FIRST: &str = "Data\\UI\\Screens\\SWF\\first.gfx";
const SECOND: &str = "Data\\UI\\Screens\\SWF\\second.gfx";

#[test]
fn two_imports_of_one_name_from_different_movies_keep_their_own_fonts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(
                CLIENT,
                client_of(
                    stage(),
                    &[("first.gfx", 9, "$Font"), ("second.gfx", 10, "$Font")],
                    &[(9, 0), (10, 1_000)],
                ),
            ),
            TestEntry::zstd(FIRST, font_movie_with(glyph(NARROW), "$Font")),
            TestEntry::zstd(SECOND, font_movie_with(glyph(WIDE), "$Font")),
        ],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert!(render.skipped.is_empty(), "{:?}", render.skipped);
    assert!(render.imports.unresolved.is_empty(), "{:?}", render.imports);
    assert_eq!(render.imports.movies, [FIRST, SECOND]);
    let pixels = render.image.pixels();
    let (upper, lower) = (green_pixels(pixels, 0..60), green_pixels(pixels, 60..100));
    assert!(upper > 0, "the first font draws");
    assert!(
        lower > 3 * upper,
        "the second font is four times as wide: {lower} against {upper}"
    );
    Ok(())
}

#[test]
fn the_same_import_twice_is_resolved_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(
                CLIENT,
                client_of(
                    stage(),
                    &[("first.gfx", 9, "$Font"), ("FIRST.gfx", 10, "$FONT")],
                    &[(9, 0), (10, 1_000)],
                ),
            ),
            TestEntry::zstd(FIRST, font_movie_with(glyph(NARROW), "$Font")),
        ],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert!(render.skipped.is_empty(), "{:?}", render.skipped);
    assert_eq!(render.imports.movies, [FIRST]);
    let pixels = render.image.pixels();
    assert_eq!(
        green_pixels(pixels, 0..60),
        green_pixels(pixels, 60..100),
        "both fields draw with the one font"
    );
    Ok(())
}

#[test]
fn a_font_the_movie_exports_itself_does_not_replace_the_imported_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let own = stage()
        .tag(75, &font3_body(4, "Own", &[glyph(WIDE)]))
        .tag(56, &exports_body(&[(4, "$Font")]));
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(
                CLIENT,
                client_of(own, &[("first.gfx", 9, "$Font")], &[(9, 0), (4, 1_000)]),
            ),
            TestEntry::zstd(FIRST, font_movie_with(glyph(NARROW), "$Font")),
        ],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert!(render.skipped.is_empty(), "{:?}", render.skipped);
    assert!(render.imports.unresolved.is_empty(), "{:?}", render.imports);
    let pixels = render.image.pixels();
    let (imported, own) = (green_pixels(pixels, 0..60), green_pixels(pixels, 60..100));
    assert!(imported > 0, "the imported font draws");
    assert!(
        own > 3 * imported,
        "the own font stays the wide one: {own} against {imported}"
    );
    Ok(())
}

#[test]
fn an_unresolved_import_stays_a_missing_font_beside_an_own_export_of_its_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let own = stage()
        .tag(75, &font3_body(4, "Own", &[glyph(WIDE)]))
        .tag(56, &exports_body(&[(4, "$Font")]));
    let archive = archive(
        dir.path(),
        vec![TestEntry::zstd(
            CLIENT,
            client_of(own, &[("gone.gfx", 9, "$Font")], &[(9, 0)]),
        )],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert_eq!(render.imports.unresolved.len(), 1, "{:?}", render.imports);
    assert_eq!(render.imports.unresolved[0].issue, ImportIssue::NotFound);
    assert_eq!(render.skipped.len(), 1, "{:?}", render.skipped);
    assert_eq!(render.skipped[0].reason.code(), "missing_font");
    Ok(())
}

#[test]
fn a_name_one_movie_lacks_is_reported_for_that_url_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(
                CLIENT,
                client_of(
                    stage(),
                    &[("first.gfx", 9, "$Font"), ("second.gfx", 10, "$Font")],
                    &[(9, 0), (10, 1_000)],
                ),
            ),
            TestEntry::zstd(FIRST, font_movie_with(glyph(NARROW), "$Font")),
            TestEntry::zstd(SECOND, font_movie_with(glyph(WIDE), "$Other")),
        ],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert_eq!(render.imports.unresolved.len(), 1, "{:?}", render.imports);
    assert_eq!(render.imports.unresolved[0].url, "second.gfx");
    assert_eq!(
        render.imports.unresolved[0].issue,
        ImportIssue::NoSuchExport
    );
    assert_eq!(render.skipped.len(), 1, "{:?}", render.skipped);
    assert_eq!(render.skipped[0].reason.code(), "missing_font");
    Ok(())
}
