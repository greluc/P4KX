//! Hostile movies: nesting, mask counts and canvases beyond the limits end in a report or a typed
//! error, never in a panic, a hang or an unbounded allocation.

use std::collections::HashMap;

use p4kx_gfx::{
    Background, CanvasSize, MAX_SCENE_MASKS, MAX_SPRITE_DEPTH, Movie, RasterOptions, SkipReason,
};
use p4kx_testkit::{
    TestFill, TestMovie, TestPlace, TestShape, TestShapeRecord, TestSignature, morph_shape_body,
    sprite_body,
};

const RED: [u8; 4] = [255, 0, 0, 255];

const fn stage() -> TestMovie {
    TestMovie::new(TestSignature::Gfx).stage(100, 100)
}

fn rect(id: u16, (width, height): (i32, i32), fill: TestFill) -> TestShape {
    TestShape::new(id, 3, [0, width, 0, height])
        .fill(fill)
        .record(TestShapeRecord::Move {
            to: (0, 0),
            fill0: 0,
            fill1: 1,
            line: 0,
        })
        .record(TestShapeRecord::Line(width, 0))
        .record(TestShapeRecord::Line(0, height))
        .record(TestShapeRecord::Line(-width, 0))
        .record(TestShapeRecord::Line(0, -height))
}

fn compose(bytes: Vec<u8>) -> p4kx_gfx::Scene {
    let movie = Movie::parse(bytes).unwrap_or_else(|error| unreachable!("invalid movie: {error}"));
    let assets = movie.assets();
    movie.first_frame(&assets)
}

#[test]
fn a_sprite_that_places_itself_is_cut_off_at_the_depth_limit() {
    let square = rect(4, (20, 20), TestFill::Solid(RED));
    let looping = sprite_body(
        10,
        1,
        &[
            (26, TestPlace::new(1, 4).body2()),
            (26, TestPlace::new(2, 10).body2()),
            (1, Vec::new()),
        ],
    );
    let scene = compose(
        stage()
            .tag(square.code(), &square.body())
            .tag(39, &looping)
            .tag(26, &TestPlace::new(1, 10).body2())
            .tag(1, &[])
            .bytes(),
    );
    assert!(
        scene.items.len() <= MAX_SPRITE_DEPTH as usize + 1,
        "{} items",
        scene.items.len()
    );
}

#[test]
fn a_chain_of_sprites_deeper_than_the_limit_is_cut_off_and_listed() {
    let square = rect(4, (20, 20), TestFill::Solid(RED));
    let mut movie = stage().tag(square.code(), &square.body());
    let mut child = 4;
    const { assert!(MAX_SPRITE_DEPTH < 30, "the chain is longer than the limit") };
    for id in 10..40 {
        movie = movie.tag(
            39,
            &sprite_body(
                id,
                1,
                &[(26, TestPlace::new(1, child).body2()), (1, Vec::new())],
            ),
        );
        child = id;
    }
    let scene = compose(
        movie
            .tag(26, &TestPlace::new(1, child).body2())
            .tag(1, &[])
            .bytes(),
    );
    assert!(scene.items.is_empty(), "the square lies beyond the limit");
    assert!(
        scene
            .skipped
            .iter()
            .any(|skipped| skipped.reason == SkipReason::SpriteDepth),
        "{:?}",
        scene.skipped
    );
}

#[test]
fn more_masks_than_the_limit_are_skipped_and_listed() {
    let square = rect(4, (20, 20), TestFill::Solid(RED));
    let mut movie = stage().tag(square.code(), &square.body());
    assert_eq!(MAX_SCENE_MASKS, 256, "the test places 40 masks more");
    for depth in 1..=296 {
        movie = movie.tag(26, &TestPlace::new(depth, 4).clip(depth).body2());
    }
    let scene = compose(movie.tag(1, &[]).bytes());
    assert_eq!(scene.masks.len(), MAX_SCENE_MASKS, "capped");
    let refused = scene
        .skipped
        .iter()
        .filter(|skipped| skipped.reason == SkipReason::TooManyMasks)
        .count();
    assert_eq!(refused, 40, "every further mask listed");
}

#[test]
fn a_stage_beyond_the_pixel_limit_is_a_typed_error_at_every_size() {
    let scene = compose(stage().stage(40_000, 40_000).tag(1, &[]).bytes());
    for size in [
        CanvasSize::Stage,
        CanvasSize::Width(1 << 20),
        CanvasSize::Fit {
            width: u32::MAX,
            height: u32::MAX,
        },
    ] {
        let options = RasterOptions {
            size,
            background: Background::Transparent,
        };
        assert!(
            scene.rasterize(&HashMap::new(), &options).is_err(),
            "{size:?}"
        );
    }
}

#[test]
fn nested_fan_out_of_stage_sized_fills_stops_at_the_fill_budget() {
    let square = rect(4, (2000, 2000), TestFill::Solid([255, 0, 0, 128]));
    let mut movie = stage().tag(square.code(), &square.body());
    let mut child = 4;
    for id in 10..13 {
        let mut placements: Vec<(u16, Vec<u8>)> = (1..=16)
            .map(|depth| (26, TestPlace::new(depth, child).body2()))
            .collect();
        placements.push((1, Vec::new()));
        movie = movie.tag(39, &sprite_body(id, 1, &placements));
        child = id;
    }
    let bytes = movie
        .tag(26, &TestPlace::new(1, child).body2())
        .tag(1, &[])
        .bytes();
    let scene = compose(bytes);
    assert_eq!(scene.items.len(), 4096, "composed");
    let options = RasterOptions {
        size: CanvasSize::Fit {
            width: 512,
            height: 512,
        },
        background: Background::Transparent,
    };
    let raster = scene
        .rasterize(&HashMap::new(), &options)
        .unwrap_or_else(|error| unreachable!("cannot render: {error}"));
    let stopped = raster
        .skipped
        .iter()
        .filter(|skipped| skipped.reason == SkipReason::FillBudget)
        .count();
    assert_eq!(stopped, 1, "{:?}", raster.skipped.len());
}

#[test]
fn a_morph_placed_at_many_ratios_stops_at_the_conversion_budget() {
    let start: Vec<TestShapeRecord> = std::iter::once(TestShapeRecord::Move {
        to: (0, 0),
        fill0: 0,
        fill1: 0,
        line: 0,
    })
    .chain(std::iter::repeat_n(TestShapeRecord::Line(10, 0), 2000))
    .collect();
    let body = morph_shape_body(5, ([255, 0, 0, 255], [0, 0, 255, 255]), &start, &start);
    let mut movie = stage().tag(46, &body);
    for depth in 1..=10_000 {
        movie = movie.tag(26, &TestPlace::new(depth, 5).ratio(depth).body2());
    }
    let scene = compose(movie.tag(1, &[]).bytes());
    assert!(
        scene
            .skipped
            .iter()
            .any(|skipped| skipped.reason == SkipReason::TooManyRecords),
        "{} conversions",
        scene.shapes.len()
    );
    assert!(
        scene.shapes.len() < 10_000,
        "{} conversions",
        scene.shapes.len()
    );
}
