// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Untrusted textures and SVG files: size, path, nesting and entity limits, and what is listed.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ImageSources, Limits, MAX_SVG_BYTES, ScreenComposer};
use p4kx_testkit::screens_image_sample;

use crate::images::{CANVAS, texture, vector};
use crate::support::{TestResult, near, pixel, stage};

fn compose_svg(
    svg: impl Fn(&str) -> Result<Vec<u8>, String>,
    limits: Limits,
) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = screens_image_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut composer = ScreenComposer::new(&forge)
        .with_images(ImageSources {
            texture: &texture,
            vector: &svg,
        })
        .with_limits(limits);
    Ok(composer.compose(CANVAS, None)?)
}

fn reasons(composed: &Composed, element: &str) -> Vec<String> {
    composed
        .report()
        .unsupported
        .iter()
        .filter(|entry| entry.element == element)
        .map(|entry| entry.reason.clone())
        .collect()
}

fn single(svg: &str) -> impl Fn(&str) -> Result<Vec<u8>, String> {
    let bytes = svg.as_bytes().to_vec();
    move |_| Ok(bytes.clone())
}

fn unreadable(composed: &Composed) -> Vec<String> {
    composed
        .report()
        .unsupported
        .iter()
        .filter(|entry| entry.reason == "vector graphic missing or unreadable")
        .flat_map(|entry| entry.examples.clone())
        .collect()
}

#[test]
fn an_svg_file_beyond_the_size_limit_is_listed_and_skipped() -> TestResult {
    let big = vec![b' '; MAX_SVG_BYTES + 1];
    let composed = compose_svg(move |_| Ok(big.clone()), Limits::DEFAULT)?;
    let examples = unreadable(&composed);
    assert!(
        examples.iter().any(|text| text.contains("bytes")),
        "{examples:?}"
    );
    let raster = composed.rasterize(&stage())?;
    assert_eq!(
        pixel(&raster.image, 10, 70),
        [0, 0, 0, 0],
        "nothing is drawn"
    );
    Ok(())
}

#[test]
fn a_path_with_too_many_commands_is_refused() -> TestResult {
    let mut data = String::from("M0 0");
    data.push_str(&" l1 1".repeat(300_000));
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="{data}"/></svg>"#
    );
    let composed = compose_svg(single(&svg), Limits::DEFAULT)?;
    let examples = unreadable(&composed);
    assert!(
        examples.iter().any(|text| text.contains("commands")),
        "{examples:?}"
    );
    Ok(())
}

#[test]
fn entity_declarations_are_refused_and_not_expanded() -> TestResult {
    let svg = r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "AAAA"><!ENTITY b "&a;&a;&a;&a;">]><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="&b;" height="10"/></svg>"#;
    let composed = compose_svg(single(svg), Limits::DEFAULT)?;
    let examples = unreadable(&composed);
    assert!(
        examples.iter().any(|text| text.contains("entities")),
        "{examples:?}"
    );
    Ok(())
}

#[test]
fn numbers_that_are_not_finite_or_out_of_range_are_refused() -> TestResult {
    for data in [
        "M 1e999 0 L 1 1",
        "M 0 0 L 1e9 1",
        "M 0 0 L nan 1",
        "M 0 0 Q 1",
    ] {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="{data}" stroke="#fff"/></svg>"##
        );
        let composed = compose_svg(single(&svg), Limits::DEFAULT)?;
        let examples = unreadable(&composed);
        assert!(
            examples.iter().any(|text| text.contains("malformed")),
            "{data}: {examples:?}"
        );
    }
    Ok(())
}

#[test]
fn a_document_without_a_size_or_not_svg_is_refused() -> TestResult {
    for svg in [
        r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="1" height="1"/></svg>"#,
        r"<html/>",
        "not xml at all",
    ] {
        let composed = compose_svg(single(svg), Limits::DEFAULT)?;
        assert!(!unreadable(&composed).is_empty(), "{svg}");
    }
    Ok(())
}

#[test]
fn deeply_nested_groups_are_cut_and_listed() -> TestResult {
    let depth = 40;
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">{}<rect width="10" height="10" fill="#fff"/>{}</svg>"##,
        "<g>".repeat(depth),
        "</g>".repeat(depth)
    );
    let composed = compose_svg(single(&svg), Limits::DEFAULT)?;
    assert!(
        reasons(&composed, "WidgetCustomShape")
            .iter()
            .any(|reason| reason.contains("nested deeper")),
        "{:?}",
        composed.report().unsupported
    );
    Ok(())
}

#[test]
fn unsupported_features_are_listed_once_and_the_rest_is_drawn() -> TestResult {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 10 10">
<defs><linearGradient id="g"><stop offset="0" stop-color="#00ff00"/><stop offset="1" stop-color="#ff0000"/></linearGradient></defs>
<image xlink:href="http://example.invalid/x.png" width="10" height="10"/>
<rect width="10" height="10" fill="url(#g)" clip-path="url(#c)"/></svg>"##;
    let composed = compose_svg(single(svg), Limits::DEFAULT)?;
    let reasons = reasons(&composed, "WidgetCustomShape");
    for expected in ["clip paths", "image and text", "paint servers"] {
        assert!(
            reasons.iter().any(|reason| reason.contains(expected)),
            "{expected}: {reasons:?}"
        );
    }
    let raster = composed.rasterize(&stage())?;
    assert!(
        near(pixel(&raster.image, 40, 65), [0, 255, 0, 255], 3),
        "the gradient is drawn in its first colour"
    );
    Ok(())
}

#[test]
fn the_path_budget_of_a_screen_stops_further_graphics() -> TestResult {
    let limits = Limits {
        vector_commands: 12,
        ..Limits::DEFAULT
    };
    let composed = compose_svg(vector, limits)?;
    let examples = unreadable(&composed);
    assert!(
        examples.iter().any(|text| text.contains("path budget")),
        "{examples:?}"
    );
    assert!(
        composed.report().images_drawn > 0,
        "the first graphics are drawn"
    );
    Ok(())
}

#[test]
fn use_elements_cannot_multiply_a_document_without_bound() -> TestResult {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><defs><g id="a">{}</g></defs>{}</svg>"#,
        "<g/>".repeat(3000),
        r##"<use href="#a"/>"##.repeat(400)
    );
    let composed = compose_svg(single(&svg), Limits::DEFAULT)?;
    let examples = unreadable(&composed);
    assert!(
        examples.iter().any(|text| text.contains("elements")),
        "{examples:?}"
    );
    Ok(())
}

#[test]
fn copies_for_nine_slices_and_slots_count_against_the_path_budget() -> TestResult {
    let limits = Limits {
        vector_commands: 25,
        ..Limits::DEFAULT
    };
    let composed = compose_svg(vector, limits)?;
    assert!(
        reasons(&composed, "WidgetCustomShape")
            .iter()
            .any(|reason| reason.contains("copies of vector graphics")),
        "{:?}",
        composed.report().unsupported
    );
    Ok(())
}

#[test]
fn the_pixel_budget_of_a_screen_stops_further_textures() -> TestResult {
    let bytes = screens_image_sample().build();
    let forge = DataForge::open(&bytes)?;
    let limits = Limits {
        texture_pixels: 3,
        ..Limits::DEFAULT
    };
    let composed = ScreenComposer::new(&forge)
        .with_images(ImageSources {
            texture: &texture,
            vector: &vector,
        })
        .with_limits(limits)
        .compose(CANVAS, None)?;
    let skipped = composed
        .report()
        .unsupported
        .iter()
        .find(|entry| entry.reason == "texture missing or unreadable")
        .ok_or("no texture is listed")?;
    assert!(
        skipped
            .examples
            .iter()
            .any(|text| text.contains("2 x 2 pixels")),
        "{:?}",
        skipped.examples
    );
    assert!(
        composed.scene().bitmap_ids().is_empty(),
        "no bitmap is kept"
    );
    Ok(())
}

#[test]
fn a_texture_that_cannot_be_read_names_the_reason() -> TestResult {
    let bytes = screens_image_sample().build();
    let forge = DataForge::open(&bytes)?;
    let broken = |_: &str, _: u32| Err("the header is truncated".to_owned());
    let composed = ScreenComposer::new(&forge)
        .with_images(ImageSources {
            texture: &broken,
            vector: &vector,
        })
        .compose(CANVAS, None)?;
    let entry = composed
        .report()
        .unsupported
        .iter()
        .find(|entry| entry.reason == "texture missing or unreadable")
        .ok_or("not listed")?;
    assert!(entry.examples.iter().any(|text| text.contains("truncated")));
    assert!(
        entry.count >= 7,
        "every image widget is skipped: {}",
        entry.count
    );
    Ok(())
}
