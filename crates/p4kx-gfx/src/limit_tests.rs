//! The limits of composing and rasterising, exercised with small values through the private
//! functions the public ones pass their constants to.

use std::collections::HashMap;

use p4kx_testkit::{
    TestFill, TestMovie, TestPlace, TestShape, TestShapeRecord, TestSignature, morph_shape_body,
};

use crate::geometry::{Rect, Rgba};
use crate::image::RgbaImage;
use crate::movie::Movie;
use crate::place::ColorTransform;
use crate::raster::{Background, CanvasSize, Raster, RasterOptions};
use crate::raster_budget::RasterLimits;
use crate::scene::{
    Affine, BitmapPaint, Cap, Join, LayerKind, Origin, Paint, PathCommand, Scene, SceneItem,
    SceneMask, ScenePath, ScenePoint, SceneShape, ShapeLayer, StrokeStyle,
};
use crate::scene_compose::{Composer, SceneLimits};
use crate::scene_report::SkipReason;

pub(crate) const RED: Rgba = Rgba {
    red: 255,
    green: 0,
    blue: 0,
    alpha: 255,
};

const SIDE: f32 = 400.0;

pub(crate) fn point(x: f32, y: f32) -> ScenePoint {
    ScenePoint { x, y }
}

pub(crate) fn square() -> ScenePath {
    ScenePath {
        commands: vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(SIDE, 0.0)),
            PathCommand::LineTo(point(SIDE, SIDE)),
            PathCommand::LineTo(point(0.0, SIDE)),
            PathCommand::Close,
        ],
    }
}

pub(crate) fn fill(paint: Paint) -> SceneShape {
    SceneShape {
        id: 1,
        layers: vec![ShapeLayer {
            path: square(),
            paint,
            kind: LayerKind::Fill,
        }],
    }
}

pub(crate) fn item(mask: Option<usize>, color_transform: ColorTransform) -> SceneItem {
    SceneItem {
        shape: 0,
        transform: Affine::IDENTITY,
        color_transform,
        mask,
        origin: Origin {
            id: 1,
            depth: 1,
            sprite: None,
        },
    }
}

pub(crate) fn scene(shape: SceneShape, items: Vec<SceneItem>, masks: Vec<SceneMask>) -> Scene {
    Scene {
        stage: Rect {
            x_min: 0,
            x_max: 400,
            y_min: 0,
            y_max: 400,
        },
        shapes: vec![shape],
        items,
        masks,
        ..Scene::default()
    }
}

fn rasterize(scene: &Scene, bitmaps: &HashMap<u16, RgbaImage>, limits: &RasterLimits) -> Raster {
    let options = RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    };
    scene
        .rasterize_with(bitmaps, &options, limits)
        .unwrap_or_else(|error| unreachable!("cannot rasterise: {error}"))
}

fn reasons(raster: &Raster) -> Vec<SkipReason> {
    raster
        .skipped
        .iter()
        .map(|skipped| skipped.reason)
        .collect()
}

fn alpha(raster: &Raster, x: usize, y: usize) -> u8 {
    raster.image.pixels()[(y * 20 + x) * 4 + 3]
}

pub(crate) fn bitmaps() -> HashMap<u16, RgbaImage> {
    let image =
        RgbaImage::from_pixels(2, 2, vec![255; 16]).unwrap_or_else(|error| unreachable!("{error}"));
    HashMap::from([(7, image)])
}

pub(crate) fn bitmap_scene(color_transform: ColorTransform) -> Scene {
    let paint = Paint::Bitmap(BitmapPaint {
        bitmap_id: 7,
        transform: Affine::IDENTITY,
        repeating: true,
        smoothed: false,
    });
    scene(fill(paint), vec![item(None, color_transform)], Vec::new())
}

pub(crate) const TINT: ColorTransform = ColorTransform {
    multiply: [1.0; 4],
    add: [10.0, 0.0, 0.0, 0.0],
};

#[test]
fn the_fill_budget_stops_drawing_and_is_listed() {
    let scene = scene(
        fill(Paint::Solid(RED)),
        vec![item(None, ColorTransform::IDENTITY); 10],
        Vec::new(),
    );
    let roomy = rasterize(&scene, &HashMap::new(), &RasterLimits::DEFAULT);
    assert!(roomy.skipped.is_empty(), "{:?}", roomy.skipped);
    let limits = RasterLimits {
        coverage_factor: 3,
        ..RasterLimits::DEFAULT
    };
    let raster = rasterize(&scene, &HashMap::new(), &limits);
    assert_eq!(
        reasons(&raster),
        [SkipReason::FillBudget],
        "once, then stopped"
    );
    assert_eq!(alpha(&raster, 10, 10), 255, "what fit is drawn");
}

#[test]
fn a_mask_over_the_memory_limit_skips_its_items() {
    let masks = vec![SceneMask {
        shapes: vec![(0, Affine::IDENTITY)],
        parent: None,
    }];
    let scene = scene(
        fill(Paint::Solid(RED)),
        vec![item(Some(0), ColorTransform::IDENTITY)],
        masks,
    );
    let fits = RasterLimits {
        mask_bytes: 400,
        ..RasterLimits::DEFAULT
    };
    let raster = rasterize(&scene, &HashMap::new(), &fits);
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
    assert_eq!(alpha(&raster, 10, 10), 255, "masked fill");
    let tight = RasterLimits {
        mask_bytes: 399,
        ..RasterLimits::DEFAULT
    };
    let raster = rasterize(&scene, &HashMap::new(), &tight);
    assert_eq!(reasons(&raster), [SkipReason::MaskMemory]);
    assert_eq!(alpha(&raster, 10, 10), 0, "not drawn");
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
    let raster = rasterize(&scene, &HashMap::new(), &limits);
    assert_eq!(reasons(&raster), [SkipReason::FillBudget]);
}

#[test]
fn bitmap_copies_beyond_the_limit_skip_their_layer() {
    let scene = bitmap_scene(TINT);
    let fits = RasterLimits {
        bitmap_pixels: 4,
        ..RasterLimits::DEFAULT
    };
    let raster = rasterize(&scene, &bitmaps(), &fits);
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
    let tight = RasterLimits {
        bitmap_pixels: 3,
        ..RasterLimits::DEFAULT
    };
    let raster = rasterize(&scene, &bitmaps(), &tight);
    assert_eq!(
        reasons(&raster),
        [SkipReason::BitmapMemory],
        "transformed copy"
    );
    let plain = bitmap_scene(ColorTransform::IDENTITY);
    let raster = rasterize(&plain, &bitmaps(), &tight);
    assert_eq!(reasons(&raster), [SkipReason::BitmapMemory], "plain copy");
}

#[test]
fn a_hairline_in_a_mask_clips_to_one_pixel_line() {
    let hairline = SceneShape {
        id: 2,
        layers: vec![ShapeLayer {
            path: ScenePath {
                commands: vec![
                    PathCommand::MoveTo(point(0.0, 200.0)),
                    PathCommand::LineTo(point(SIDE, 200.0)),
                ],
            },
            paint: Paint::Solid(RED),
            kind: LayerKind::Stroke(StrokeStyle {
                width: 0.0,
                cap: Cap::Butt,
                join: Join::Miter,
                miter_limit: 3.0,
            }),
        }],
    };
    let mut scene = scene(
        fill(Paint::Solid(RED)),
        vec![item(Some(0), ColorTransform::IDENTITY)],
        vec![SceneMask {
            shapes: vec![(1, Affine::IDENTITY)],
            parent: None,
        }],
    );
    scene.shapes.push(hairline);
    let raster = rasterize(&scene, &HashMap::new(), &RasterLimits::DEFAULT);
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
    assert!(alpha(&raster, 10, 9) > 0, "the line lies on rows 9 and 10");
    assert!(alpha(&raster, 10, 10) > 0, "the line lies on rows 9 and 10");
    assert_eq!(alpha(&raster, 10, 3), 0, "elsewhere clipped");
    assert_eq!(alpha(&raster, 10, 16), 0, "elsewhere clipped");
}

fn movie() -> TestMovie {
    TestMovie::new(TestSignature::Gfx).stage(100, 100)
}

fn square_tag(id: u16) -> TestShape {
    TestShape::new(id, 3, [0, 20, 0, 20])
        .fill(TestFill::Solid([255, 0, 0, 255]))
        .record(TestShapeRecord::Move {
            to: (0, 0),
            fill0: 0,
            fill1: 1,
            line: 0,
        })
        .record(TestShapeRecord::Line(20, 0))
        .record(TestShapeRecord::Line(0, 20))
        .record(TestShapeRecord::Line(-20, 0))
        .record(TestShapeRecord::Line(0, -20))
}

fn compose(bytes: Vec<u8>, limits: SceneLimits) -> Scene {
    let movie = Movie::parse(bytes).unwrap_or_else(|error| unreachable!("invalid movie: {error}"));
    let assets = movie.assets();
    Composer::compose_with(&assets, movie.header().frame_size, limits)
}

fn squares(count: u16) -> Vec<u8> {
    let shape = square_tag(4);
    let mut movie = movie().tag(shape.code(), &shape.body());
    for depth in 1..=count {
        movie = movie.tag(26, &TestPlace::new(depth, 4).body2());
    }
    movie.tag(1, &[]).bytes()
}

#[test]
fn layers_beyond_the_path_limit_stop_the_composition() {
    let limits = SceneLimits {
        paths: 3,
        ..SceneLimits::DEFAULT
    };
    let scene = compose(squares(6), limits);
    assert_eq!(scene.items.len(), 3);
    let listed: Vec<_> = scene.skipped.iter().map(|skipped| skipped.reason).collect();
    assert_eq!(listed, [SkipReason::TooManyPaths]);
    let roomy = compose(squares(6), SceneLimits::DEFAULT);
    assert_eq!(roomy.items.len(), 6);
}

#[test]
fn path_commands_beyond_the_limit_stop_the_composition() {
    let limits = SceneLimits {
        segments: 1,
        ..SceneLimits::DEFAULT
    };
    let scene = compose(squares(6), limits);
    assert_eq!(scene.items.len(), 0);
    let listed: Vec<_> = scene.skipped.iter().map(|skipped| skipped.reason).collect();
    assert_eq!(listed, [SkipReason::TooManyPaths]);
}

fn morphs(ratios: u16, records: usize) -> Vec<u8> {
    let start: Vec<TestShapeRecord> = std::iter::once(TestShapeRecord::Move {
        to: (0, 0),
        fill0: 0,
        fill1: 0,
        line: 0,
    })
    .chain(std::iter::repeat_n(TestShapeRecord::Line(10, 0), records))
    .collect();
    let body = morph_shape_body(5, ([255, 0, 0, 255], [0, 0, 255, 255]), &start, &start);
    let mut movie = movie().tag(46, &body);
    for depth in 1..=ratios {
        movie = movie.tag(26, &TestPlace::new(depth, 5).ratio(depth).body2());
    }
    movie.tag(1, &[]).bytes()
}

#[test]
fn converting_a_morph_per_ratio_is_charged_before_the_conversion() {
    let limits = SceneLimits {
        records: 40,
        ..SceneLimits::DEFAULT
    };
    let scene = compose(morphs(30, 8), limits);
    assert!(
        scene.shapes.iter().all(|shape| shape.layers.is_empty()),
        "no style is selected, no layer is made"
    );
    assert!(
        (1..=4).contains(&scene.shapes.len()),
        "{} conversions",
        scene.shapes.len()
    );
    let listed: Vec<_> = scene.skipped.iter().map(|skipped| skipped.reason).collect();
    assert_eq!(listed, [SkipReason::TooManyRecords]);
    let roomy = compose(morphs(30, 8), SceneLimits::DEFAULT);
    assert_eq!(roomy.shapes.len(), 30);
    assert!(roomy.skipped.is_empty(), "{:?}", roomy.skipped);
}
