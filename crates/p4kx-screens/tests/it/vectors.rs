// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Vector graphics of the synthetic image canvas: custom shapes, icons, nine-slices, colour
//! overlays, the cascade and the output formats.

use p4kx_screens::ScreenComposer;

use crate::images::{CANVAS, composed};
use crate::support::{TestResult, near, pixel, stage};

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

#[test]
fn a_custom_shape_is_drawn_in_the_overlay_colour() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 10, 70), BLUE, 2),
        "the white square becomes blue"
    );
    assert_eq!(pixel(image, 25, 70), CLEAR, "nothing beyond the square");
    Ok(())
}

#[test]
fn stretching_fills_the_widget_with_the_graphic() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 40, 65), RED, 2),
        "the red band, from an attribute"
    );
    assert!(
        near(pixel(image, 60, 65), GREEN, 2),
        "the green band, from a style rule"
    );
    assert!(
        near(pixel(image, 80, 65), BLUE, 2),
        "the blue band, from a style attribute"
    );
    assert_eq!(
        pixel(image, 95, 65),
        CLEAR,
        "the graphic ends with the widget"
    );
    Ok(())
}

#[test]
fn a_nine_slice_keeps_its_caps_and_stretches_the_middle() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(near(pixel(image, 105, 65), RED, 2), "the left cap");
    assert!(
        near(pixel(image, 109, 65), RED, 2),
        "which is ten pixels wide"
    );
    assert!(near(pixel(image, 112, 65), GREEN, 2), "then the middle");
    assert!(
        near(pixel(image, 178, 65), GREEN, 2),
        "which takes the rest"
    );
    assert!(near(pixel(image, 185, 65), BLUE, 2), "and the right cap");
    Ok(())
}

#[test]
fn arcs_circles_and_strokes_are_drawn() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 10, 90), RED, 4),
        "the disc made of two arcs"
    );
    assert_eq!(pixel(image, 14, 90), CLEAR, "between the disc and the ring");
    assert!(
        near(pixel(image, 18, 90), WHITE, 4),
        "the stroke of the circle"
    );
    assert_eq!(pixel(image, 2, 82), CLEAR, "outside the ring");
    Ok(())
}

#[test]
fn an_image_widget_can_show_a_vector_graphic() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    assert!(
        near(pixel(&raster.image, 40, 90), [255, 255, 0, 255], 2),
        "the svg path and colour of the fill of the image widget"
    );
    Ok(())
}

#[test]
fn an_icon_shows_its_custom_graphic_in_its_colour() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    assert!(
        near(pixel(&raster.image, 160, 40), [0, 255, 255, 255], 2),
        "the icon"
    );
    Ok(())
}

#[test]
fn an_icon_preset_names_its_graphic_by_the_naming_convention() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    assert!(
        near(pixel(&raster.image, 185, 40), [0, 255, 255, 255], 2),
        "the preset TestBoxX is test_box_x.svg"
    );
    Ok(())
}

#[test]
fn the_cascade_sets_the_path_and_the_colour_of_a_shape() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    assert!(
        near(pixel(&raster.image, 70, 90), GREEN, 2),
        "a style entry names the graphic and fills it"
    );
    Ok(())
}

#[test]
fn a_slot_named_in_the_id_takes_its_colour_from_the_palette() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    assert!(
        near(pixel(&raster.image, 160, 90), [255, 170, 0, 128], 3),
        "Accent1 at half opacity"
    );
    Ok(())
}

#[test]
fn a_use_element_places_the_element_it_names() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let image = &raster.image;
    assert!(
        near(pixel(image, 185, 90), GREEN, 2),
        "the dot, moved to 3, 3"
    );
    assert_eq!(
        pixel(image, 177, 82),
        CLEAR,
        "the definition itself is not drawn"
    );
    Ok(())
}

#[test]
fn the_svg_output_holds_the_shapes_of_the_graphics() -> TestResult {
    let composed = composed()?;
    let mut svg = Vec::new();
    composed.write_svg(&stage(), Some("image_sample"), &mut svg)?;
    let text = String::from_utf8(svg)?;
    assert!(
        text.matches("<path").count() >= 10,
        "the graphics are paths: {}",
        text.matches("<path").count()
    );
    assert!(!text.contains("<script"), "nothing active is written");
    Ok(())
}

#[test]
fn drawn_widgets_are_not_listed_as_without_renderer() -> TestResult {
    let composed = composed()?;
    let report = composed.report();
    assert!(
        report
            .unsupported
            .iter()
            .all(|entry| !entry.reason.contains("no renderer")),
        "{:?}",
        report.unsupported
    );
    assert!(report.images_drawn >= 12, "{}", report.images_drawn);
    Ok(())
}

#[test]
fn without_sources_images_and_shapes_are_listed() -> TestResult {
    let bytes = p4kx_testkit::screens_image_sample().build();
    let forge = p4kx_dataforge::DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(CANVAS, None)?;
    let report = composed.report();
    for element in ["WidgetImage", "WidgetCustomShape", "WidgetIcon"] {
        assert!(
            report
                .unsupported
                .iter()
                .any(|entry| entry.element == element && entry.reason.contains("no renderer")),
            "{element} is listed"
        );
    }
    assert_eq!(report.images_drawn, 0, "nothing is drawn");
    assert!(report.images_left_out > 0, "and it is counted");
    Ok(())
}
