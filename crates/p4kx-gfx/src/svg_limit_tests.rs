//! The SVG writer charges the budgets of the rasteriser: the same limits end the frame at the
//! same item with the same report.

use std::collections::HashMap;

use crate::image::RgbaImage;
use crate::limit_tests::{RED, TINT, bitmap_scene, bitmaps, fill, item, scene};
use crate::place::ColorTransform;
use crate::raster::{Background, CanvasSize, RasterOptions};
use crate::raster_budget::RasterLimits;
use crate::scene::{Affine, Paint, Scene, SceneMask};
use crate::scene_report::{SkipReason, Skipped};

const OPTIONS: RasterOptions = RasterOptions {
    size: CanvasSize::Stage,
    background: Background::Transparent,
};

fn svg(
    scene: &Scene,
    bitmaps: &HashMap<u16, RgbaImage>,
    limits: &RasterLimits,
) -> (String, Vec<Skipped>) {
    let mut out = Vec::new();
    let report = scene
        .write_svg_with(bitmaps, &OPTIONS, None, &mut out, limits)
        .unwrap_or_else(|error| unreachable!("cannot write: {error}"));
    (String::from_utf8(out).unwrap_or_default(), report.skipped)
}

fn raster_skipped(
    scene: &Scene,
    bitmaps: &HashMap<u16, RgbaImage>,
    limits: &RasterLimits,
) -> Vec<Skipped> {
    scene
        .rasterize_with(bitmaps, &OPTIONS, limits)
        .unwrap_or_else(|error| unreachable!("cannot rasterise: {error}"))
        .skipped
}

fn uses(svg: &str) -> usize {
    svg.matches("<use ").count()
}

#[test]
fn the_fill_budget_ends_the_frame_at_the_item_the_raster_ends_at() {
    let scene = scene(
        fill(Paint::Solid(RED)),
        vec![item(None, ColorTransform::IDENTITY); 10],
        Vec::new(),
    );
    let (roomy, skipped) = svg(&scene, &HashMap::new(), &RasterLimits::DEFAULT);
    assert!(skipped.is_empty(), "{skipped:?}");
    assert_eq!(uses(&roomy), 10, "every item");
    for factor in [1, 3, 5] {
        let limits = RasterLimits {
            coverage_factor: factor,
            ..RasterLimits::DEFAULT
        };
        let (limited, skipped) = svg(&scene, &HashMap::new(), &limits);
        assert_eq!(skipped, raster_skipped(&scene, &HashMap::new(), &limits));
        assert_eq!(
            skipped.iter().map(|entry| entry.reason).collect::<Vec<_>>(),
            [SkipReason::FillBudget],
            "once, then stopped"
        );
        assert_eq!(
            uses(&limited),
            usize::try_from(factor).unwrap_or(0),
            "the items that fit"
        );
    }
}

#[test]
fn building_masks_is_charged_to_the_fill_budget() {
    let masks = vec![SceneMask {
        shapes: vec![(0, Affine::IDENTITY)],
        parent: None,
    }];
    let scene = scene(
        fill(Paint::Solid(RED)),
        vec![item(Some(0), ColorTransform::IDENTITY)],
        masks,
    );
    let limits = RasterLimits {
        coverage_factor: 1,
        ..RasterLimits::DEFAULT
    };
    let (document, skipped) = svg(&scene, &HashMap::new(), &limits);
    assert_eq!(skipped, raster_skipped(&scene, &HashMap::new(), &limits));
    assert_eq!(
        skipped.iter().map(|entry| entry.reason).collect::<Vec<_>>(),
        [SkipReason::FillBudget]
    );
    assert!(!document.contains("<mask"), "no mask for a skipped item");
    assert_eq!(uses(&document), 0, "nothing placed");
}

#[test]
fn bitmap_copies_beyond_the_limit_skip_their_layer() {
    let scene = bitmap_scene(TINT);
    for pixels in [3, 4] {
        let limits = RasterLimits {
            bitmap_pixels: pixels,
            ..RasterLimits::DEFAULT
        };
        let (document, skipped) = svg(&scene, &bitmaps(), &limits);
        assert_eq!(skipped, raster_skipped(&scene, &bitmaps(), &limits));
        assert_eq!(
            document.contains("data:image/png;base64,"),
            skipped.is_empty(),
            "embedded only when it fits"
        );
    }
}

#[test]
fn a_missing_bitmap_skips_its_layer_like_the_raster() {
    let scene = bitmap_scene(ColorTransform::IDENTITY);
    let (document, skipped) = svg(&scene, &HashMap::new(), &RasterLimits::DEFAULT);
    assert_eq!(
        skipped,
        raster_skipped(&scene, &HashMap::new(), &RasterLimits::DEFAULT)
    );
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(!document.contains("<image"), "nothing to embed");
}
