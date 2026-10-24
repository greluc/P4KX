// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Texts of the synthetic text canvas: font, size, colour, alignment, wrapping, localisation,
//! the cascade, automatic width and what is listed.

use p4kx_dataforge::DataForge;
use p4kx_gfx::{Font, Localise, Movie};
use p4kx_screens::{Composed, Report, ScreenComposer, TextSources};
use p4kx_testkit::{screens_font_movie, screens_text_sample, text_localisation};

use crate::support::{TestResult, near, pixel, stage};

const CANVAS: &str = "libs/foundry/records/ui/buildingblocks/test/text_sample";
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

pub(crate) fn fonts() -> Result<Vec<Font>, Box<dyn std::error::Error>> {
    let movie = Movie::parse(screens_font_movie())?;
    Ok(movie.assets().fonts)
}

pub(crate) fn compose(
    fonts: &[Font],
    localise: Option<Localise<'_>>,
) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut composer = ScreenComposer::new(&forge).with_texts(TextSources { fonts, localise });
    Ok(composer.compose(CANVAS, None)?)
}

fn composed() -> Result<Composed, Box<dyn std::error::Error>> {
    let fonts = fonts()?;
    compose(&fonts, Some(&text_localisation))
}

fn reasons(report: &Report) -> Vec<String> {
    report
        .unsupported
        .iter()
        .map(|entry| format!("{}: {}", entry.element, entry.reason))
        .collect()
}

#[test]
fn texts_are_drawn_in_the_colour_size_and_font_of_the_widget() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 2, 4), RED, 2), "the A of Test Sans");
    assert!(near(pixel(image, 8, 6), RED, 2), "the B beside it");
    assert_eq!(pixel(image, 8, 2), CLEAR, "the B is half as tall as the A");
    assert_eq!(pixel(image, 5, 2), CLEAR, "the advance leaves a gap");
    Ok(())
}

#[test]
fn texts_align_left_right_and_centre_in_their_box() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 96, 24), RED, 2), "right aligned");
    assert_eq!(pixel(image, 2, 24), CLEAR, "not at the left edge");
    assert!(near(pixel(image, 49, 44), RED, 2), "centred");
    assert_eq!(pixel(image, 2, 44), CLEAR, "not at the left edge either");
    Ok(())
}

#[test]
fn a_wrapping_text_breaks_after_the_last_space_that_fits() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 102, 4), RED, 2),
        "first A of the first line"
    );
    assert!(
        near(pixel(image, 108, 4), RED, 2),
        "second A of the first line"
    );
    assert_eq!(pixel(image, 113, 4), CLEAR, "no third A on the first line");
    assert!(
        near(pixel(image, 102, 14), RED, 2),
        "first A of the second line"
    );
    assert!(
        near(pixel(image, 108, 14), RED, 2),
        "second A of the second line"
    );
    Ok(())
}

#[test]
fn a_localisation_key_resolves_in_the_language() -> TestResult {
    let composed = composed()?;
    let raster = composed.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 2, 64), RED, 2),
        "the A of the text of the key"
    );
    assert!(
        near(pixel(image, 8, 66), RED, 2),
        "the B of the text of the key"
    );
    assert!(
        !reasons(composed.report())
            .iter()
            .any(|reason| reason.contains("localisation")),
        "nothing about keys is listed: {:?}",
        reasons(composed.report())
    );
    Ok(())
}

#[test]
fn a_key_the_language_lacks_is_drawn_as_it_is_and_listed() -> TestResult {
    let fonts = fonts()?;
    let lookup = |_: &str| None;
    let composed = compose(&fonts, Some(&lookup))?;
    let entry = composed
        .report()
        .unsupported
        .iter()
        .find(|entry| entry.reason.contains("not in the language"))
        .ok_or("the missing key is not listed")?;
    assert_eq!(
        (
            entry.element.as_str(),
            entry.count,
            entry.examples.as_slice()
        ),
        ("WidgetText", 1, ["key".to_owned()].as_slice())
    );
    let raster = composed.rasterize(&stage())?;
    assert_eq!(
        pixel(&raster.image, 8, 66),
        CLEAR,
        "the key itself is drawn, with empty boxes for the characters the font lacks"
    );
    Ok(())
}

#[test]
fn keys_are_drawn_as_they_are_when_no_language_is_given() -> TestResult {
    let fonts = fonts()?;
    let composed = compose(&fonts, None)?;
    assert!(
        reasons(composed.report())
            .iter()
            .any(|reason| reason.contains("no language given")),
        "{:?}",
        reasons(composed.report())
    );
    Ok(())
}

#[test]
fn a_character_without_a_glyph_is_an_empty_box_and_listed() -> TestResult {
    let composed = composed()?;
    let raster = composed.rasterize(&stage())?;
    let image = &raster.image;
    assert!(pixel(image, 101, 54)[3] > 60, "the left edge of the box");
    assert_eq!(pixel(image, 103, 54), CLEAR, "the box is empty");
    let entry = composed
        .report()
        .unsupported
        .iter()
        .find(|entry| entry.reason.contains("U+005A"))
        .ok_or("the missing glyph is not listed")?;
    assert_eq!(entry.examples, ["missing"]);
    Ok(())
}

#[test]
fn the_cascade_sets_size_colour_and_alignment_over_the_widget() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 93, 88), GREEN, 2),
        "twice the size, green and right aligned by the style entry"
    );
    assert_eq!(pixel(image, 2, 88), CLEAR, "no red text at the left edge");
    Ok(())
}

#[test]
fn a_text_of_automatic_width_is_as_wide_as_its_text() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 132, 84), RED, 2), "the A of the text");
    assert!(
        near(pixel(image, 144, 90), GREEN, 2),
        "the card starts where the text ends"
    );
    assert_eq!(pixel(image, 140, 95), CLEAR, "and not before it");
    Ok(())
}

#[test]
fn drawn_and_left_out_texts_are_counted() -> TestResult {
    let composed = composed()?;
    let report = composed.report();
    assert_eq!((report.texts_drawn, report.texts_left_out), (8, 0));
    assert!(
        report.shapes > 8,
        "every glyph is a placement: {}",
        report.shapes
    );
    Ok(())
}

#[test]
fn without_fonts_every_text_is_left_out_and_listed() -> TestResult {
    let bytes = screens_text_sample().build();
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(CANVAS, None)?;
    let report = composed.report();
    assert_eq!((report.texts_drawn, report.texts_left_out), (0, 8));
    let entry = report
        .unsupported
        .iter()
        .find(|entry| entry.element == "WidgetText")
        .ok_or("the texts are not listed")?;
    assert_eq!(entry.count, 8);
    Ok(())
}

#[test]
fn the_scene_of_texts_is_the_same_every_time() -> TestResult {
    let first = composed()?;
    let second = composed()?;
    assert_eq!(first.scene(), second.scene());
    assert_eq!(first.report(), second.report());
    Ok(())
}

#[test]
fn the_svg_draws_glyphs_as_paths() -> TestResult {
    let mut svg = Vec::new();
    composed()?.write_svg(&stage(), Some("text_sample"), &mut svg)?;
    let text = String::from_utf8(svg)?;
    assert!(text.contains("<path"), "glyph outlines are paths");
    assert!(!text.contains("<text"), "no <text> element names a font");
    Ok(())
}
