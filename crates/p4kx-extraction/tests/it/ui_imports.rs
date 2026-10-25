// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Fonts that a UI movie imports from other movies of the Archive: resolution by export name,
//! URL matching, chains, cycles and the movies the import URL names.

use p4kx_extraction::{ConversionError, ImportIssue, UiMovie};
use p4kx_testkit::{TestEntry, exports_body, import_assets_body};

use crate::ui_import_support::{
    CLIENT, FONTS, GREEN, TestResult, archive, client, font_movie, options, pixel, stage,
};

#[test]
fn an_imported_font_draws_the_text_with_the_glyphs_of_the_exporting_movie() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(
                CLIENT,
                client("../../../fonts/Shared/fonts_en.swf", "$Orbitron"),
            ),
            TestEntry::zstd(FONTS, font_movie("$Orbitron")),
        ],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert!(render.skipped.is_empty(), "{:?}", render.skipped);
    assert_eq!(pixel(render.image.pixels(), (6, 36)), GREEN, "a glyph");
    assert!(render.imports.unresolved.is_empty(), "{:?}", render.imports);
    assert_eq!(
        render.imports.movies,
        [FONTS],
        "the .gfx twin of the .swf URL"
    );
    Ok(())
}

#[test]
fn unresolved_imports_are_listed_and_their_texts_drawn_as_boxes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(CLIENT, client("fonts_en.gfx", "$Gone")),
            TestEntry::zstd(FONTS, font_movie("$Orbitron")),
            TestEntry::zstd(
                "Data\\UI\\Screens\\SWF\\Other.swf",
                client("missing.gfx", "$Orbitron"),
            ),
        ],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert_eq!(render.skipped.len(), 1, "{:?}", render.skipped);
    assert_eq!(render.skipped[0].reason.code(), "missing_font");
    assert_eq!(render.imports.unresolved.len(), 1, "{:?}", render.imports);
    assert_eq!(
        render.imports.unresolved[0].issue,
        ImportIssue::NoSuchExport
    );
    let other =
        UiMovie::from_archive(&archive, "Data\\UI\\Screens\\SWF\\Other.swf")?.render(&options())?;
    assert_eq!(other.imports.unresolved[0].issue, ImportIssue::NotFound);
    assert_eq!(ImportIssue::NotFound.code(), "not_found");
    assert_eq!(ImportIssue::Unreadable(String::new()).code(), "unreadable");
    assert_eq!(ImportIssue::NoSuchExport.code(), "no_such_export");
    assert_eq!(ImportIssue::TooDeep.code(), "too_deep");
    assert_eq!(ImportIssue::OverBudget.code(), "over_budget");
    assert!(other.imports.movies.is_empty(), "no movie read");
    Ok(())
}

#[test]
fn a_font_exported_under_an_imported_name_is_followed_and_cycles_end() -> TestResult {
    let dir = tempfile::tempdir()?;
    let relay = stage()
        .tag(
            71,
            &import_assets_body("fonts_en.gfx", 2, &[(4, "$Orbitron")]),
        )
        .tag(56, &exports_body(&[(4, "$Relay")]))
        .bytes();
    let first = stage()
        .tag(71, &import_assets_body("b.swf", 2, &[(4, "$Loop")]))
        .tag(56, &exports_body(&[(4, "$Loop")]))
        .bytes();
    let second = stage()
        .tag(71, &import_assets_body("a.swf", 2, &[(4, "$Loop")]))
        .tag(56, &exports_body(&[(4, "$Loop")]))
        .bytes();
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(CLIENT, client("relay.swf", "$Relay")),
            TestEntry::zstd("Data\\UI\\Screens\\SWF\\relay.swf", relay),
            TestEntry::zstd(
                "Data\\UI\\Screens\\SWF\\fonts_en.gfx",
                font_movie("$Orbitron"),
            ),
            TestEntry::zstd("Data\\UI\\Screens\\SWF\\Loop.swf", client("a.swf", "$Loop")),
            TestEntry::zstd("Data\\UI\\Screens\\SWF\\a.swf", first),
            TestEntry::zstd("Data\\UI\\Screens\\SWF\\b.swf", second),
        ],
    )?;
    let render = UiMovie::from_archive(&archive, CLIENT)?.render(&options())?;
    assert!(render.skipped.is_empty(), "{:?}", render.skipped);
    assert_eq!(
        pixel(render.image.pixels(), (6, 36)),
        GREEN,
        "through the relay"
    );
    let looping =
        UiMovie::from_archive(&archive, "Data\\UI\\Screens\\SWF\\Loop.swf")?.render(&options())?;
    assert_eq!(looping.imports.unresolved.len(), 1, "{:?}", looping.imports);
    assert_eq!(looping.imports.unresolved[0].issue, ImportIssue::TooDeep);
    Ok(())
}

#[test]
fn the_movie_an_import_names_opens_from_an_archive_and_from_disk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![
            TestEntry::zstd(
                CLIENT,
                client("../../fonts/Shared/fonts_en.swf", "$Orbitron"),
            ),
            TestEntry::zstd(FONTS, font_movie("$Orbitron")),
        ],
    )?;
    let client_movie = UiMovie::from_archive(&archive, CLIENT)?;
    let source = client_movie.import_source("../../fonts/Shared/fonts_en.swf")?;
    assert_eq!(source.name(), FONTS);
    assert!(matches!(
        client_movie.import_source("nowhere.gfx"),
        Err(ConversionError::UnknownEntry(_))
    ));
    let folder = dir.path().join("swf");
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join("Panel.gfx"), client("fonts.gfx", "$Orbitron"))?;
    std::fs::write(folder.join("fonts.gfx"), font_movie("$Orbitron"))?;
    let on_disk = UiMovie::from_file(&folder.join("Panel.gfx"))?;
    let imported = on_disk.import_source("fonts.gfx")?;
    assert!(
        imported.name().ends_with("fonts.gfx"),
        "{}",
        imported.name()
    );
    for url in [
        "/etc/fonts.gfx",
        "\\etc\\passwd",
        "C:x.swf",
        "C:\\x.swf",
        "../../x",
        "..\\x.gfx",
        "a/../../x.gfx",
        "fonts.gfx:stream",
        "",
        "..",
    ] {
        assert!(
            matches!(
                on_disk.import_source(url),
                Err(ConversionError::UnknownEntry(_))
            ),
            "{url}"
        );
    }
    assert_eq!(
        on_disk.import_source("sub/../fonts.gfx")?.name(),
        imported.name(),
        "a dot-dot that stays inside the folder resolves"
    );
    Ok(())
}
