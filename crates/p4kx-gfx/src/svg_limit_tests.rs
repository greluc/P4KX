//! The SVG writer charges the budgets of the rasteriser: the same limits end the frame at the
//! same item with the same report.

use std::collections::HashMap;

use crate::GfxError;
use crate::image::RgbaImage;
use crate::limit_tests::{RED, TINT, bitmap_scene, bitmaps, fill, item, point, scene, square};
use crate::place::ColorTransform;
use crate::raster::{Background, CanvasSize, RasterOptions};
use crate::raster_budget::RasterLimits;
use crate::scene::{
    Affine, BitmapPaint, Cap, Join, LayerKind, Paint, PathCommand, Scene, SceneMask, ScenePath,
    SceneShape, ShapeLayer, StrokeStyle,
};
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

fn bitmap_layer(bitmap_id: u16, path: ScenePath) -> ShapeLayer {
    ShapeLayer {
        path,
        paint: Paint::Bitmap(BitmapPaint {
            bitmap_id,
            transform: Affine::IDENTITY,
            repeating: true,
            smoothed: false,
        }),
        kind: LayerKind::Fill,
    }
}

fn overflowing_square() -> ScenePath {
    let far = 1.0e38;
    ScenePath {
        commands: vec![
            PathCommand::MoveTo(point(-far, -far)),
            PathCommand::LineTo(point(far, -far)),
            PathCommand::LineTo(point(far, far)),
            PathCommand::LineTo(point(-far, far)),
            PathCommand::Close,
        ],
    }
}

fn two_bitmaps() -> HashMap<u16, RgbaImage> {
    let mut all = bitmaps();
    let other =
        RgbaImage::from_pixels(2, 2, vec![128; 16]).unwrap_or_else(|error| unreachable!("{error}"));
    all.insert(8, other);
    all
}

fn scaled(factor: f32) -> Affine {
    Affine {
        a: factor,
        d: factor,
        ..Affine::IDENTITY
    }
}

#[test]
fn a_layer_the_raster_skips_for_its_geometry_is_not_written_or_charged() {
    let shape = SceneShape {
        id: 1,
        layers: vec![
            bitmap_layer(7, square()),
            bitmap_layer(8, overflowing_square()),
        ],
    };
    let mut scaled_up = item(None, ColorTransform::IDENTITY);
    scaled_up.transform = scaled(10_000.0);
    let scene = scene(shape, vec![scaled_up], Vec::new());
    for pixels in [8, 4] {
        let limits = RasterLimits {
            bitmap_pixels: pixels,
            ..RasterLimits::DEFAULT
        };
        let (document, skipped) = svg(&scene, &two_bitmaps(), &limits);
        assert_eq!(skipped, raster_skipped(&scene, &two_bitmaps(), &limits));
        let reasons: Vec<_> = skipped.iter().map(|entry| entry.reason).collect();
        assert_eq!(
            reasons,
            [SkipReason::InvalidGeometry],
            "the skipped layer is not charged a copy, so the memory limit of {pixels} is no issue"
        );
        assert_eq!(document.matches("<image ").count(), 1, "the drawn bitmap");
        assert_eq!(document.matches("<path").count(), 1, "the drawn layer");
        assert_eq!(
            document.matches("<use xlink:href=\"#s").count(),
            1,
            "the item is placed"
        );
    }
}

#[test]
fn a_stroke_with_unbounded_reach_is_skipped_like_the_raster() {
    let reach = StrokeStyle {
        width: f32::INFINITY,
        cap: Cap::Butt,
        join: Join::Miter,
        miter_limit: 3.0,
    };
    let shape = SceneShape {
        id: 1,
        layers: vec![
            ShapeLayer {
                path: square(),
                paint: Paint::Solid(RED),
                kind: LayerKind::Fill,
            },
            ShapeLayer {
                path: square(),
                paint: Paint::Solid(RED),
                kind: LayerKind::Stroke(reach),
            },
        ],
    };
    let scene = scene(
        shape,
        vec![item(None, ColorTransform::IDENTITY)],
        Vec::new(),
    );
    let (document, skipped) = svg(&scene, &HashMap::new(), &RasterLimits::DEFAULT);
    assert_eq!(
        skipped,
        raster_skipped(&scene, &HashMap::new(), &RasterLimits::DEFAULT)
    );
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert_eq!(skipped[0].reason, SkipReason::InvalidGeometry);
    assert_eq!(document.matches("<path").count(), 1, "only the fill");
    assert!(!document.contains("stroke"), "no stroke");
}

#[test]
fn the_byte_budget_ends_the_document_with_a_typed_error() {
    let scene = scene(
        fill(Paint::Solid(RED)),
        vec![item(None, ColorTransform::IDENTITY); 3],
        Vec::new(),
    );
    let (document, _) = svg(&scene, &HashMap::new(), &RasterLimits::DEFAULT);
    let exact = u64::try_from(document.len()).unwrap_or(u64::MAX);
    let write = |svg_bytes| {
        let limits = RasterLimits {
            svg_bytes,
            ..RasterLimits::DEFAULT
        };
        let mut out = Vec::new();
        scene
            .write_svg_with(&HashMap::new(), &OPTIONS, None, &mut out, &limits)
            .map(|_| out.len())
    };
    assert_eq!(write(exact).ok(), Some(document.len()), "fits exactly");
    assert!(
        matches!(
            write(exact - 1),
            Err(GfxError::SvgTooLarge { limit }) if limit == exact - 1
        ),
        "one byte less"
    );
    assert!(matches!(write(0), Err(GfxError::SvgTooLarge { .. })));
}

#[test]
fn the_byte_budget_also_stops_an_embedded_bitmap() {
    let scene = bitmap_scene(ColorTransform::IDENTITY);
    let limits = RasterLimits {
        svg_bytes: 200,
        ..RasterLimits::DEFAULT
    };
    let mut out = Vec::new();
    let result = scene.write_svg_with(&bitmaps(), &OPTIONS, None, &mut out, &limits);
    assert!(
        matches!(result, Err(GfxError::SvgTooLarge { limit: 200 })),
        "{result:?}"
    );
    assert!(out.len() <= 200, "nothing is written past the budget");
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
