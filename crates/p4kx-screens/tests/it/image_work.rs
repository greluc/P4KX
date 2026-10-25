// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Images under transforms and clips, `use` elements, and the work an SVG file may cost.

use std::cell::Cell;

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ImageSources, Limits, ScreenComposer};
use p4kx_testkit::{TestCanvas, TestDim, TestValue, TestWidget};

use crate::images::texture;
use crate::support::{
    SCREEN, TestResult, database, empty_at, guid_text, near, pixel, red_at, stage,
};

const SVG: &str = "UI/Test/work.svg";

fn canvas() -> TestCanvas {
    TestCanvas::new(SCREEN, "image_work", 100.0, 50.0)
}

fn picture(name: &str, width: f32, height: f32) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetImage", name)
        .size(TestDim::Fixed(width), TestDim::Fixed(height))
        .with("imagePath", TestValue::text("UI/Test/quad.tif"))
}

fn shape(name: &str, width: f32, height: f32) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetCustomShape", name)
        .size(TestDim::Fixed(width), TestDim::Fixed(height))
        .with("svgPath", TestValue::text(SVG))
        .with("renderShape", TestValue::Bool(true))
        .with("scalingBehavior", TestValue::text("StretchXY"))
}

fn compose_with(
    canvas: &TestCanvas,
    svg: &str,
    texture: &dyn Fn(&str, u32, u64) -> Result<p4kx_texture::RgbaImage, String>,
) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = database(std::iter::once(canvas.record()));
    let forge = DataForge::open(&bytes)?;
    let vector = |_: &str| Ok(svg.as_bytes().to_vec());
    Ok(ScreenComposer::new(&forge)
        .with_images(ImageSources {
            texture,
            vector: &vector,
        })
        .with_limits(Limits::DEFAULT)
        .compose(&guid_text(SCREEN), None)?)
}

fn svg_screen(svg: &str) -> Result<Composed, Box<dyn std::error::Error>> {
    compose_with(&canvas().widget(shape("work", 20.0, 20.0)), svg, &texture)
}

fn failures(composed: &Composed) -> Vec<String> {
    composed
        .report()
        .unsupported
        .iter()
        .filter(|entry| entry.reason == "vector graphic missing or unreadable")
        .flat_map(|entry| entry.examples.clone())
        .collect()
}

fn notes(composed: &Composed) -> Vec<String> {
    composed
        .report()
        .unsupported
        .iter()
        .filter(|entry| entry.element == "WidgetCustomShape")
        .map(|entry| entry.reason.clone())
        .collect()
}

#[test]
fn a_turned_image_widget_is_drawn_turned() -> TestResult {
    let turned = picture("bar", 40.0, 10.0)
        .anchor(0.5, 0.5)
        .pivot(0.5, 0.5)
        .rotate(90.0);
    let composed = compose_with(&canvas().widget(turned), "", &texture)?;
    let image = composed.rasterize(&stage())?.image;
    assert_ne!(pixel(&image, 50, 40), [0, 0, 0, 0], "the bar stands up");
    assert_ne!(pixel(&image, 50, 10), [0, 0, 0, 0], "to both ends");
    assert!(empty_at(&image, 35, 25), "where it lay before");
    assert!(empty_at(&image, 65, 25), "on both sides");
    Ok(())
}

#[test]
fn an_image_widget_is_cut_at_the_clip_of_its_parent() -> TestResult {
    let window = TestWidget::display("window")
        .size(TestDim::Fixed(20.0), TestDim::Fixed(20.0))
        .overflow("Clip");
    let wide = picture("wide", 60.0, 10.0).parent("window");
    let composed = compose_with(&canvas().widget(window).widget(wide), "", &texture)?;
    let image = composed.rasterize(&stage())?.image;
    assert_ne!(pixel(&image, 10, 5), [0, 0, 0, 0], "inside the window");
    assert!(empty_at(&image, 30, 5), "outside the window");
    Ok(())
}

#[test]
fn a_vector_widget_is_cut_at_the_clip_of_its_parent() -> TestResult {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#f00"/></svg>"##;
    let window = TestWidget::display("window")
        .size(TestDim::Fixed(20.0), TestDim::Fixed(20.0))
        .overflow("Clip");
    let wide = shape("wide", 60.0, 10.0).parent("window");
    let composed = compose_with(&canvas().widget(window).widget(wide), svg, &texture)?;
    let image = composed.rasterize(&stage())?.image;
    assert!(red_at(&image, 10, 5), "inside the window");
    assert!(empty_at(&image, 30, 5), "outside the window");
    Ok(())
}

#[test]
fn the_texture_loader_is_given_what_is_left_of_the_pixel_budget() -> TestResult {
    let allowed = Cell::new(0);
    let recording = |path: &str, side: u32, pixels: u64| {
        allowed.set(pixels);
        texture(path, side, pixels)
    };
    let limits = canvas().widget(picture("one", 10.0, 10.0));
    compose_with(&limits, "", &recording)?;
    assert_eq!(allowed.get(), Limits::DEFAULT.texture_pixels.min(1 << 25));
    Ok(())
}

fn chain(levels: usize, fan: usize) -> String {
    let body: String = (0..levels)
        .map(|level| {
            let uses = format!(r##"<use href="#g{}"/>"##, level + 1).repeat(fan);
            format!(r#"<g id="g{level}">{uses}</g>"#)
        })
        .chain(std::iter::once(format!(r#"<g id="g{levels}"/>"#)))
        .collect();
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><defs>{body}</defs><use href="#g0"/></svg>"##
    )
}

#[test]
fn an_exponential_chain_of_use_elements_is_refused() -> TestResult {
    let composed = svg_screen(&chain(10, 8))?;
    let examples = failures(&composed);
    assert!(
        examples
            .iter()
            .any(|text| text.contains("too many elements")),
        "{examples:?}"
    );
    Ok(())
}

#[test]
fn style_rules_that_match_everything_do_not_multiply_the_work_of_a_chain() -> TestResult {
    let rules = "* { fill: #f00; stroke: #fff; stroke-width: 1; opacity: 0.9 }".repeat(200);
    let svg = chain(10, 8).replace("<defs>", &format!("<style>{rules}</style><defs>"));
    let composed = svg_screen(&svg)?;
    let examples = failures(&composed);
    assert!(
        examples
            .iter()
            .any(|text| text.contains("too many elements")),
        "{examples:?}"
    );
    Ok(())
}

#[test]
fn a_huge_path_attribute_used_again_and_again_is_refused() -> TestResult {
    let data = format!("M0 0 L10 10{}", " ".repeat(1_900_000));
    let uses = r##"<use href="#p"/>"##.repeat(12);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><defs><path id="p" d="{data}"/></defs>{uses}</svg>"#
    );
    let composed = svg_screen(&svg)?;
    let examples = failures(&composed);
    assert!(
        examples
            .iter()
            .any(|text| text.contains("too many elements")),
        "{examples:?}"
    );
    Ok(())
}

#[test]
fn a_use_element_that_names_itself_is_cut_and_listed() -> TestResult {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><defs><g id="a"><use href="#a"/></g></defs><rect width="10" height="10" fill="#f00"/><use href="#a"/></svg>"##;
    let composed = svg_screen(svg)?;
    assert!(failures(&composed).is_empty(), "{:?}", failures(&composed));
    assert!(
        notes(&composed)
            .iter()
            .any(|reason| reason.contains("nested deeper")),
        "{:?}",
        composed.report().unsupported
    );
    let image = composed.rasterize(&stage())?.image;
    assert!(red_at(&image, 10, 10), "the rest is drawn");
    Ok(())
}

#[test]
fn a_use_element_of_a_symbol_fits_the_view_box_to_its_size() -> TestResult {
    let symbol = r##"<symbol id="s" viewBox="0 0 10 10"><rect width="10" height="10" fill="#f00"/></symbol>"##;
    let drawn = |attributes: &str| {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10"><defs>{symbol}</defs><use href="#s" {attributes}/></svg>"##
        );
        svg_screen(&svg).and_then(|composed| Ok(composed.rasterize(&stage())?.image))
    };
    let stretched = drawn(r#"width="20" height="10""#)?;
    assert!(red_at(&stretched, 3, 10), "the left half");
    assert!(red_at(&stretched, 17, 10), "and the right half");
    let natural = drawn("")?;
    assert!(red_at(&natural, 3, 10), "the left half");
    assert!(
        near(pixel(&natural, 17, 10), [0, 0, 0, 0], 0),
        "but not the right half"
    );
    Ok(())
}
