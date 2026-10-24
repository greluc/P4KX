// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Composing and drawing the synthetic MFD canvas.

use p4kx_dataforge::DataForge;
use p4kx_screens::ScreenComposer;
use p4kx_testkit::{SCREEN_MFD, screens_sample};

use crate::support::{TestResult, guid_text, near, pixel, stage};

const MFD: &str = "libs/foundry/records/ui/buildingblocks/test/test_mfd";

#[test]
fn the_mfd_is_laid_out_and_painted_in_the_palette_of_the_brand() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(MFD, Some("test"))?;
    assert_eq!(
        (composed.width(), composed.height()),
        (200, 100),
        "the canvas size"
    );
    let raster = composed.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 100, 90), [10, 16, 24, 255], 1),
        "the root's own background"
    );
    assert!(
        near(pixel(image, 100, 10), [15, 88, 112, 255], 2),
        "the card the style entry fills with half of the Base slot"
    );
    assert!(
        near(pixel(image, 20, 44), [255, 170, 0, 255], 2),
        "the circle in Accent1"
    );
    assert!(
        near(pixel(image, 58, 46), [255, 255, 255, 255], 2),
        "the triangle"
    );
    assert!(
        near(pixel(image, 84, 32), [240, 240, 240, 255], 2),
        "the badge a tag selector fills with the Bright slot"
    );
    assert!(
        near(pixel(image, 131, 45), [200, 30, 30, 255], 2),
        "the border of the nested panel, laid out by the grow of its widget"
    );
    assert!(
        near(pixel(image, 150, 45), [26, 160, 52, 255], 3),
        "the translucent fill inside the nested panel, over the root"
    );
    assert!(
        !near(pixel(image, 100, 90), [255, 0, 255, 255], 20),
        "a hidden widget is not drawn"
    );
    Ok(())
}

#[test]
fn what_has_no_renderer_is_reported_with_its_widgets() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(MFD, Some("test"))?;
    let report = composed.report();
    assert!(
        !report.is_complete(),
        "unsupported widgets make the render incomplete"
    );
    let note = |element: &str, reason: &str| {
        report
            .unsupported
            .iter()
            .find(|entry| entry.element == element && entry.reason.contains(reason))
    };
    let text = note("WidgetText", "no renderer").ok_or("the text is not reported")?;
    assert_eq!(
        (text.count, text.examples.as_slice()),
        (1, ["label".to_owned()].as_slice())
    );
    assert!(
        note("WidgetImage", "no renderer").is_some(),
        "the image is reported"
    );
    assert!(
        note("WidgetCustomShape", "no renderer").is_some(),
        "a widget of the nested canvas is reported"
    );
    assert!(
        note("WidgetCircle", "no renderer").is_none(),
        "a drawn type is not reported"
    );
    assert_eq!(report.canvases, 2, "the MFD canvas and the nested panel");
    assert!(
        report.shapes >= 8,
        "the shapes drawn are counted: {}",
        report.shapes
    );
    Ok(())
}

#[test]
fn the_scene_is_the_same_for_a_guid_and_a_record_path() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut composer = ScreenComposer::new(&forge);
    let by_path = composer.compose(MFD, Some("test"))?;
    let by_guid = composer.compose(&guid_text(SCREEN_MFD), Some("test"))?;
    assert_eq!(
        by_path.scene(),
        by_guid.scene(),
        "the scene does not depend on how the canvas is named"
    );
    Ok(())
}

#[test]
fn the_svg_has_one_path_per_shape() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(MFD, Some("test"))?;
    let mut svg = Vec::new();
    let written = composed.write_svg(&stage(), Some("test_mfd"), &mut svg)?;
    assert_eq!(
        (written.width, written.height),
        (200, 100),
        "the document size"
    );
    let text = String::from_utf8(svg)?;
    assert_eq!(
        text.matches("<path").count(),
        composed.report().shapes,
        "every shape is written as a path"
    );
    assert!(text.contains("<title>test_mfd</title>"), "the title");
    Ok(())
}

#[test]
fn rendering_twice_gives_the_same_scene() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut composer = ScreenComposer::new(&forge);
    let first = composer.compose(MFD, Some("test"))?;
    let second = composer.compose(MFD, Some("test"))?;
    assert_eq!(
        first.scene(),
        second.scene(),
        "the composition is deterministic"
    );
    assert_eq!(first.report(), second.report(), "so is its report");
    Ok(())
}
