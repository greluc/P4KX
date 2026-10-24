// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Textures of the synthetic image canvas: scaling, flips, tint, alpha and what is listed.

use std::cell::Cell;

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ImageSources, Report, ScreenComposer};
use p4kx_testkit::{screens_image_sample, screens_texture, screens_vector};
use p4kx_texture::RgbaImage;

use crate::support::{TestResult, near, pixel, stage};

pub(crate) const CANVAS: &str = "libs/foundry/records/ui/buildingblocks/test/image_sample";
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

pub(crate) fn texture(path: &str, _side: u32) -> Result<RgbaImage, String> {
    let (width, height, pixels) = screens_texture(path).ok_or("no such Entry")?;
    RgbaImage::new(width, height, pixels).map_err(|error| error.to_string())
}

pub(crate) fn vector(path: &str) -> Result<Vec<u8>, String> {
    screens_vector(path).ok_or_else(|| "no such Entry".to_owned())
}

pub(crate) fn compose_with(
    sources: ImageSources<'_>,
) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = screens_image_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut composer = ScreenComposer::new(&forge).with_images(sources);
    Ok(composer.compose(CANVAS, None)?)
}

pub(crate) fn composed() -> Result<Composed, Box<dyn std::error::Error>> {
    compose_with(ImageSources {
        texture: &texture,
        vector: &vector,
    })
}

fn element<'a>(
    report: &'a Report,
    element: &str,
    reason: &str,
) -> Option<&'a p4kx_screens::Unsupported> {
    report
        .unsupported
        .iter()
        .find(|entry| entry.element == element && entry.reason.contains(reason))
}

#[test]
fn an_image_that_fills_stretches_the_texture_over_the_widget() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 5, 3), RED, 2), "top left quadrant");
    assert!(near(pixel(image, 35, 3), GREEN, 2), "top right quadrant");
    assert!(near(pixel(image, 5, 17), BLUE, 2), "bottom left quadrant");
    assert!(
        near(pixel(image, 35, 17), WHITE, 2),
        "bottom right quadrant"
    );
    assert_eq!(pixel(image, 45, 10), CLEAR, "nothing beside the widget");
    Ok(())
}

#[test]
fn contain_keeps_the_aspect_ratio_and_places_the_image() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert_eq!(
        pixel(image, 55, 10),
        CLEAR,
        "the free space left of the image"
    );
    assert_eq!(
        pixel(image, 85, 10),
        CLEAR,
        "the free space right of the image"
    );
    assert!(
        near(pixel(image, 65, 5), RED, 30),
        "the square starts at x = 60"
    );
    assert!(near(pixel(image, 75, 15), WHITE, 30), "and ends at x = 80");
    Ok(())
}

#[test]
fn cover_fills_the_widget_and_crops_the_image() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 110, 10), RED, 30), "left half");
    assert!(near(pixel(image, 130, 10), GREEN, 30), "right half");
    assert!(
        pixel(image, 139, 19).first().is_some_and(|red| *red < 200),
        "the lower half of the image is cropped, so its white corner is not shown"
    );
    assert_eq!(pixel(image, 105, 25), CLEAR, "nothing below the widget");
    Ok(())
}

#[test]
fn a_flipped_image_is_mirrored() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 155, 3), GREEN, 2), "red and green swap");
    assert!(near(pixel(image, 185, 3), RED, 2), "red is on the right");
    assert!(near(pixel(image, 155, 17), WHITE, 2), "blue and white swap");
    Ok(())
}

#[test]
fn the_colour_of_the_widget_tints_the_texture() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 35, 47), RED, 3), "white becomes red");
    assert!(near(pixel(image, 5, 47), [0, 0, 0, 255], 3), "blue is lost");
    Ok(())
}

#[test]
fn the_alpha_of_the_widget_makes_the_texture_translucent() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 55, 33), [255, 0, 0, 128], 3), "half red");
    Ok(())
}

#[test]
fn a_missing_texture_is_listed_and_the_widget_skipped() -> TestResult {
    let composed = composed()?;
    let report = composed.report();
    let missing = element(report, "WidgetImage", "texture missing or unreadable")
        .ok_or("the missing texture is not listed")?;
    assert_eq!(missing.count, 1, "one widget");
    assert!(
        missing
            .examples
            .iter()
            .any(|name| name.contains("missing.dds")),
        "the path is named: {:?}",
        missing.examples
    );
    let raster = composed.rasterize(&stage())?;
    assert_eq!(pixel(&raster.image, 120, 40), CLEAR, "nothing is drawn");
    assert!(report.images_left_out >= 1, "it is counted");
    Ok(())
}

#[test]
fn a_texture_is_read_once_however_often_it_is_used() -> TestResult {
    let reads = Cell::new(0_usize);
    let counting = |path: &str, side: u32| {
        reads.set(reads.get() + 1);
        texture(path, side)
    };
    let composed = compose_with(ImageSources {
        texture: &counting,
        vector: &vector,
    })?;
    assert!(composed.report().images_drawn >= 6, "images are drawn");
    let wanted = ["UI/Test/quad.dds", "UI/Test/missing.dds"].len();
    assert_eq!(reads.get(), wanted, "one read per distinct path");
    Ok(())
}

#[test]
fn the_texture_loader_is_asked_for_the_size_of_the_screen() -> TestResult {
    let asked = Cell::new(0);
    let recording = |path: &str, side: u32| {
        asked.set(side);
        texture(path, side)
    };
    compose_with(ImageSources {
        texture: &recording,
        vector: &vector,
    })?;
    assert_eq!(asked.get(), 200, "the longer side of the canvas");
    Ok(())
}

#[test]
fn a_foreground_overlay_names_its_texture_by_url() -> TestResult {
    let composed = composed()?;
    let raster = composed.rasterize(&stage())?;
    assert!(
        near(pixel(&raster.image, 125, 85), RED, 30),
        "the overlay texture"
    );
    let report = composed.report();
    let later = element(report, "WidgetForegroundOverlay", "chosen by the game")
        .ok_or("the overlay without a texture is not listed")?;
    assert_eq!(later.count, 1, "only the one without a url");
    Ok(())
}

#[test]
fn the_scene_holds_the_bitmaps_in_png_and_svg_output() -> TestResult {
    let composed = composed()?;
    assert!(
        !composed.scene().bitmap_ids().is_empty(),
        "the scene fills shapes with bitmaps"
    );
    let mut svg = Vec::new();
    let written = composed.write_svg(&stage(), None, &mut svg)?;
    let text = String::from_utf8(svg)?;
    assert!(
        text.contains("data:image/png;base64"),
        "the texture is embedded"
    );
    assert!(
        written.skipped.is_empty(),
        "nothing is skipped: {:?}",
        written.skipped
    );
    Ok(())
}
