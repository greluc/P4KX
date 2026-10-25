// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `Still` and `ScreenTexts` over a synthetic Archive with the text canvas.

use std::fs;
use std::path::PathBuf;

use p4kx_dataforge::DataForge;
use p4kx_extraction::{Archive, ArchiveImages, Language};
use p4kx_gfx::{Background, CanvasSize, RasterOptions};
use p4kx_screens::{ScreenError, ScreenTexts, Still, StillFormat, StillRequest};
use p4kx_testkit::{
    ArchiveBuilder, GlobalIniBuilder, TestEntry, screens_font_movie, screens_text_sample, test_key,
};
use tempfile::TempDir;

use crate::support::{TestResult, near, pixel};

const CANVAS: &str = "libs/foundry/records/ui/buildingblocks/test/text_sample";
const RED: [u8; 4] = [255, 0, 0, 255];

fn archive(dir: &TempDir, fonts: bool) -> Result<Archive, Box<dyn std::error::Error>> {
    let mut builder = ArchiveBuilder::new().entry(TestEntry::stored(
        "Data\\Localization\\english\\global.ini",
        GlobalIniBuilder::new().text("test_hello", "AB").build(),
    ));
    if fonts {
        builder = builder.entry(TestEntry::stored(
            "Data\\UI\\fonts\\Shared\\fonts_en.gfx",
            screens_font_movie(),
        ));
    }
    let path: PathBuf = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    Ok(Archive::open_with_key(&path, Some(test_key()))?)
}

const fn request(format: StillFormat) -> StillRequest<'static> {
    StillRequest {
        screen: CANVAS,
        brand: None,
        raster: RasterOptions {
            size: CanvasSize::Stage,
            background: Background::Transparent,
        },
        format,
    }
}

#[test]
fn a_png_still_draws_the_fonts_and_resolves_the_keys_of_the_archive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, true)?;
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;
    let texts = ScreenTexts::load(&archive, &forge, Some(Language::English));
    assert!(texts.has_fonts(), "the font movie was read");
    assert!(texts.has_texts(), "the global.ini was read");
    assert!(texts.warnings().is_empty(), "nothing was left out");

    let still = Still::render(
        &forge,
        (&texts, &ArchiveImages::new(&archive)),
        &request(StillFormat::Png),
    )?;

    let image = still.image().ok_or("a PNG still has pixels")?;
    assert_eq!(
        (still.width(), still.height()),
        (200, 100),
        "the canvas size"
    );
    assert!(near(pixel(image, 2, 4), RED, 2), "the text of the font");
    assert!(near(pixel(image, 2, 64), RED, 2), "the text of the key");
    assert_eq!(still.format(), StillFormat::Png, "the format");
    Ok(())
}

#[test]
fn an_svg_still_writes_one_path_per_shape_and_the_screen_as_title() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, true)?;
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;
    let texts = ScreenTexts::load(&archive, &forge, Some(Language::English));

    let still = Still::render(
        &forge,
        (&texts, &ArchiveImages::new(&archive)),
        &request(StillFormat::Svg),
    )?;

    assert!(still.image().is_none(), "an SVG still has no pixels");
    let svg = String::from_utf8(still.encode()?.into_owned())?;
    assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"), "SVG");
    assert!(svg.contains("<path"), "the glyphs are paths");
    assert!(svg.contains(&format!("<title>{CANVAS}</title>")), "title");
    assert_eq!((still.width(), still.height()), (200, 100), "the size");
    Ok(())
}

#[test]
fn missing_fonts_are_named_and_the_texts_are_listed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, false)?;
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;

    let texts = ScreenTexts::load(&archive, &forge, Some(Language::English));

    assert!(!texts.has_fonts(), "no font movie");
    assert_eq!(texts.warnings().len(), 1, "one source left out");
    let still = Still::render(
        &forge,
        (&texts, &ArchiveImages::new(&archive)),
        &request(StillFormat::Png),
    )?;
    assert!(
        still
            .report()
            .unsupported
            .iter()
            .any(|entry| entry.element == "WidgetText"),
        "the text widgets are listed"
    );
    assert!(!still.is_complete(), "so the still is incomplete");
    Ok(())
}

#[test]
fn a_language_the_archive_lacks_is_a_warning_not_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, true)?;
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;

    let texts = ScreenTexts::load(&archive, &forge, Some(Language::German));

    assert!(texts.has_fonts(), "the fonts are there");
    assert!(!texts.has_texts(), "no German global.ini");
    assert_eq!(texts.warnings().len(), 1, "named");
    Still::render(
        &forge,
        (&texts, &ArchiveImages::new(&archive)),
        &request(StillFormat::Png),
    )?;
    Ok(())
}

#[test]
fn writing_a_still_replaces_the_file_with_the_encoded_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, true)?;
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;
    let texts = ScreenTexts::load(&archive, &forge, Some(Language::English));
    let still = Still::render(
        &forge,
        (&texts, &ArchiveImages::new(&archive)),
        &request(StillFormat::Png),
    )?;
    let out = dir.path().join("out.png");
    fs::write(&out, b"old")?;

    still.write(&out)?;

    assert_eq!(fs::read(&out)?, still.encode()?.as_ref(), "the PNG bytes");
    assert!(
        fs::read(&out)?.starts_with(&[0x89, b'P', b'N', b'G']),
        "a PNG"
    );
    Ok(())
}

#[test]
fn an_unknown_screen_is_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, true)?;
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;
    let texts = ScreenTexts::load(&archive, &forge, None);
    let mut asked = request(StillFormat::Png);
    asked.screen = "libs/foundry/records/ui/buildingblocks/test/nothing";

    let error = Still::render(&forge, (&texts, &ArchiveImages::new(&archive)), &asked).err();

    assert!(
        matches!(error, Some(ScreenError::UnknownCanvas { .. })),
        "{error:?}"
    );
    Ok(())
}
