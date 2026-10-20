//! Fonts that a UI movie imports from other movies of the Archive: resolution by export name,
//! URL matching, chains, cycles and the movies the import URL names.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions};
use p4kx_extraction::{Archive, ConversionError, ImportIssue, UiMovie};
use p4kx_testkit::{
    ArchiveBuilder, TestEditText, TestEntry, TestGlyph, TestMovie, TestPlace, TestShapeRecord,
    TestSignature, exports_body, font3_body, import_assets_body,
};

type TestResult = Result<(), Box<dyn Error>>;

const GREEN: [u8; 4] = [0, 255, 0, 255];
const FIELD: [i32; 4] = [0, 2000, 800, 1400];
const CLIENT: &str = "Data\\UI\\Screens\\SWF\\Panel.swf";
const FONTS: &str = "Data\\UI\\fonts\\Shared\\fonts_en.gfx";

fn square() -> TestGlyph {
    TestGlyph {
        code: u16::from(b'A'),
        advance: 12_000,
        records: vec![
            TestShapeRecord::Move {
                to: (0, 0),
                fill0: 0,
                fill1: 1,
                line: 0,
            },
            TestShapeRecord::Line(10_000, 0),
            TestShapeRecord::Line(0, -14_000),
            TestShapeRecord::Line(-10_000, 0),
            TestShapeRecord::Line(0, 14_000),
        ],
    }
}

const fn stage() -> TestMovie {
    TestMovie::new(TestSignature::Gfx).stage(100, 100)
}

fn font_movie(export: &str) -> Vec<u8> {
    stage()
        .tag(75, &font3_body(4, "Orbitron", &[square()]))
        .tag(56, &exports_body(&[(4, export)]))
        .bytes()
}

fn client(url: &str, name: &str) -> Vec<u8> {
    let text = TestEditText::new(7, FIELD)
        .text("A")
        .font(9, 400)
        .color(GREEN);
    stage()
        .tag(71, &import_assets_body(url, 2, &[(9, name)]))
        .tag(37, &text.body())
        .tag(26, &TestPlace::new(1, 7).body2())
        .tag(1, &[])
        .bytes()
}

fn archive(dir: &Path, entries: Vec<TestEntry>) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut builder = ArchiveBuilder::new();
    for entry in entries {
        builder = builder.entry(entry);
    }
    builder.write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

const fn options() -> RasterOptions {
    RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    }
}

fn pixel(pixels: &[u8], (x, y): (usize, usize)) -> &[u8] {
    let at = (y * 100 + x) * 4;
    pixels.get(at..at + 4).unwrap_or_default()
}

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
    assert!(matches!(
        on_disk.import_source("/etc/fonts.gfx"),
        Err(ConversionError::UnknownEntry(_))
    ));
    Ok(())
}
