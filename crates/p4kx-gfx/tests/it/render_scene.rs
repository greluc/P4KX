//! Composing the first frame: transforms, sprites, colour transforms, masks, morph shapes, and
//! what is skipped and why.

use crate::support::{BLUE, CLEAR, RED, assert_pixel, rect, render, scene, stage};
use p4kx_gfx::{
    Assets, CanvasSize, FillStyle, Gradient, GradientStop, MAX_SCENE_PLACEMENTS, Movie, Rect, Rgba,
    Shape, SkipReason,
};
use p4kx_testkit::{
    TestFill, TestPlace, TestShape, TestShapeRecord, morph_shape_body, sprite_body,
};

const GREEN: [u8; 4] = [0, 255, 0, 255];

#[test]
fn placement_matrices_scale_rotate_and_move() {
    let square = rect(1, (500, 500), TestFill::Solid(RED));
    let bar = rect(2, (1000, 200), TestFill::Solid(BLUE));
    let bytes = stage()
        .tag(square.code(), &square.body())
        .tag(bar.code(), &bar.body())
        .tag(
            26,
            &TestPlace::new(1, 1)
                .matrix([2.0, 2.0, 0.0, 0.0], (0, 0))
                .body2(),
        )
        .tag(
            26,
            &TestPlace::new(2, 2)
                .matrix([0.0, 0.0, 1.0, -1.0], (2000, 0))
                .body2(),
        )
        .tag(1, &[])
        .bytes();
    let image = render(&bytes, CanvasSize::Stage).image;
    assert_pixel(&image, (45, 45), RED, 0);
    assert_pixel(&image, (55, 55), CLEAR, 0);
    assert_pixel(&image, (95, 30), BLUE, 0);
    assert_pixel(&image, (85, 30), CLEAR, 0);
    assert_pixel(&image, (95, 55), CLEAR, 0);
}

#[test]
fn sprites_compose_their_transforms_and_colour_transforms() {
    let square = rect(4, (1000, 1000), TestFill::Solid(RED));
    let sprite = sprite_body(
        9,
        1,
        &[
            (
                26,
                TestPlace::new(1, 4)
                    .at(200, 0)
                    .color([0.0, 1.0, 1.0, 1.0], [0, 255, 0, 0])
                    .body2(),
            ),
            (1, Vec::new()),
        ],
    );
    let bytes = stage()
        .tag(square.code(), &square.body())
        .tag(39, &sprite)
        .tag(
            26,
            &TestPlace::new(1, 9)
                .at(0, 200)
                .color([1.0, 1.0, 1.0, 0.5], [0, 0, 0, 0])
                .body2(),
        )
        .tag(1, &[])
        .bytes();
    let raster = render(&bytes, CanvasSize::Stage);
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
    assert_pixel(&raster.image, (30, 30), [0, 255, 0, 128], 1);
    assert_pixel(&raster.image, (5, 30), CLEAR, 0);
    assert_pixel(&raster.image, (30, 5), CLEAR, 0);
}

fn masked() -> Vec<u8> {
    let mask = rect(1, (1000, 1000), TestFill::Solid(RED));
    let wide = rect(2, (2000, 2000), TestFill::Solid(BLUE));
    let strip = rect(3, (2000, 500), TestFill::Solid(RED));
    let small = rect(4, (400, 400), TestFill::Solid(GREEN));
    let inner = sprite_body(
        9,
        1,
        &[
            (26, TestPlace::new(1, 3).clip(2).body2()),
            (26, TestPlace::new(2, 2).body2()),
            (1, Vec::new()),
        ],
    );
    stage()
        .tag(mask.code(), &mask.body())
        .tag(wide.code(), &wide.body())
        .tag(strip.code(), &strip.body())
        .tag(small.code(), &small.body())
        .tag(39, &inner)
        .tag(26, &TestPlace::new(1, 1).clip(3).body2())
        .tag(26, &TestPlace::new(2, 2).body2())
        .tag(26, &TestPlace::new(3, 9).at(0, 1000).body2())
        .tag(26, &TestPlace::new(4, 4).at(1200, 1200).body2())
        .tag(1, &[])
        .bytes()
}

#[test]
fn masks_clip_the_depths_they_cover_also_inside_sprites() {
    let raster = render(&masked(), CanvasSize::Stage);
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
    let image = &raster.image;
    assert_pixel(image, (25, 25), BLUE, 0);
    assert_pixel(image, (75, 25), CLEAR, 0);
    assert_pixel(image, (25, 75), CLEAR, 0);
    assert_pixel(image, (65, 65), GREEN, 0);
    let (_, scene) = scene(&masked());
    assert_eq!(scene.masks.len(), 2, "two masks");
    assert_eq!(scene.masks[1].parent, Some(0), "nested in the first");
}

#[test]
fn a_mask_inside_a_sprite_is_also_clipped_by_the_outer_mask() {
    let mask = rect(1, (1000, 2000), TestFill::Solid(RED));
    let wide = rect(2, (2000, 2000), TestFill::Solid(BLUE));
    let strip = rect(3, (2000, 500), TestFill::Solid(RED));
    let inner = sprite_body(
        9,
        1,
        &[
            (26, TestPlace::new(1, 3).clip(2).body2()),
            (26, TestPlace::new(2, 2).body2()),
            (1, Vec::new()),
        ],
    );
    let bytes = stage()
        .tag(mask.code(), &mask.body())
        .tag(wide.code(), &wide.body())
        .tag(strip.code(), &strip.body())
        .tag(39, &inner)
        .tag(26, &TestPlace::new(1, 1).clip(2).body2())
        .tag(26, &TestPlace::new(2, 9).body2())
        .tag(1, &[])
        .bytes();
    let image = render(&bytes, CanvasSize::Stage).image;
    assert_pixel(&image, (25, 10), BLUE, 0);
    assert_pixel(&image, (75, 10), CLEAR, 0);
    assert_pixel(&image, (25, 40), CLEAR, 0);
}

#[test]
fn the_strokes_of_a_mask_clip_as_well_as_its_fills() {
    let line = TestShape::new(1, 3, [0, 2000, 0, 2000])
        .line(400, RED)
        .record(TestShapeRecord::Move {
            to: (0, 1000),
            fill0: 0,
            fill1: 0,
            line: 1,
        })
        .record(TestShapeRecord::Line(2000, 0));
    let wide = rect(2, (2000, 2000), TestFill::Solid(BLUE));
    let bytes = stage()
        .tag(line.code(), &line.body())
        .tag(wide.code(), &wide.body())
        .tag(26, &TestPlace::new(1, 1).clip(2).body2())
        .tag(26, &TestPlace::new(2, 2).body2())
        .tag(1, &[])
        .bytes();
    let image = render(&bytes, CanvasSize::Stage).image;
    assert_pixel(&image, (50, 50), BLUE, 0);
    assert_pixel(&image, (50, 20), CLEAR, 0);
}

#[test]
fn a_morph_shape_is_drawn_at_the_ratio_of_its_placement() {
    let start = [
        TestShapeRecord::Move {
            to: (0, 0),
            fill0: 0,
            fill1: 1,
            line: 0,
        },
        TestShapeRecord::Line(1000, 0),
        TestShapeRecord::Line(0, 1000),
        TestShapeRecord::Line(-1000, 0),
        TestShapeRecord::Line(0, -1000),
    ];
    let end = [
        TestShapeRecord::Move {
            to: (0, 0),
            fill0: 0,
            fill1: 0,
            line: 0,
        },
        TestShapeRecord::Line(2000, 0),
        TestShapeRecord::Line(0, 2000),
        TestShapeRecord::Line(-2000, 0),
        TestShapeRecord::Line(0, -2000),
    ];
    let bytes = stage()
        .tag(46, &morph_shape_body(5, (RED, BLUE), &start, &end))
        .tag(26, &TestPlace::new(1, 5).ratio(32_768).body2())
        .tag(1, &[])
        .bytes();
    let image = render(&bytes, CanvasSize::Stage).image;
    assert_pixel(&image, (70, 70), [127, 0, 128, 255], 1);
    assert_pixel(&image, (80, 80), CLEAR, 0);
}

#[test]
fn what_cannot_be_drawn_is_skipped_and_listed_with_its_reason() {
    let square = rect(4, (1000, 1000), TestFill::Solid(RED));
    let endless = sprite_body(8, 1, &[(26, TestPlace::new(1, 8).body2())]);
    let bytes = stage()
        .tag(square.code(), &square.body())
        .tag(34, &[42, 0, 0])
        .tag(39, &endless)
        .tag(26, &TestPlace::new(1, 99).body2())
        .tag(26, &TestPlace::new(2, 42).body2())
        .tag(26, &TestPlace::new(3, 8).body2())
        .tag(70, &TestPlace::new(4, 4).blur().blend_mode(8).body3())
        .tag(70, &TestPlace::new(5, 4).visible(false).body3())
        .tag(1, &[])
        .bytes();
    let raster = render(&bytes, CanvasSize::Stage);
    let reasons: Vec<(u16, u16, Option<u16>, &str)> = raster
        .skipped
        .iter()
        .map(|skipped| {
            (
                skipped.id,
                skipped.depth,
                skipped.sprite,
                skipped.reason.code(),
            )
        })
        .collect();
    assert_eq!(
        reasons,
        [
            (99, 1, None, "unknown_definition"),
            (42, 2, None, "not_drawn"),
            (8, 1, Some(8), "sprite_depth"),
            (4, 4, None, "filters"),
            (4, 4, None, "blend_mode"),
        ],
        "in placement order; the hidden placement is no error"
    );
    assert_eq!(
        raster.skipped[1].reason.to_string(),
        "DefineButton2 definitions are not drawn",
        "named by its tag"
    );
    assert!(raster.skipped[3].reason.is_effect(), "drawn without it");
    assert_pixel(&raster.image, (25, 25), RED, 0);
}

fn nested_sprites(fan_out: u16) -> Vec<u8> {
    let square = rect(4, (20, 20), TestFill::Solid(RED));
    let mut movie = stage().tag(square.code(), &square.body());
    let mut child = 4;
    for id in [10, 11, 12] {
        let mut tags: Vec<(u16, Vec<u8>)> = (1..=fan_out)
            .map(|depth| (26, TestPlace::new(depth, child).body2()))
            .collect();
        tags.push((1, Vec::new()));
        movie = movie.tag(39, &sprite_body(id, 1, &tags));
        child = id;
    }
    movie
        .tag(26, &TestPlace::new(1, 12).body2())
        .tag(1, &[])
        .bytes()
}

#[test]
fn a_frame_beyond_the_placement_limit_stops_with_a_report() {
    let (_, big) = scene(&nested_sprites(70));
    assert!(
        big.items.len() < MAX_SCENE_PLACEMENTS,
        "stopped at {} items",
        big.items.len()
    );
    assert_eq!(
        big.skipped.last().map(|skipped| skipped.reason),
        Some(SkipReason::TooManyPlacements),
        "reported once at the end"
    );
    let (_, small) = scene(&nested_sprites(4));
    assert_eq!(small.items.len(), 64, "every square of a small tree");
    assert_eq!(small.shapes.len(), 1, "the shape converted once");
}

#[test]
fn a_gradient_above_the_stop_limit_skips_its_layer_in_every_placement() {
    let movie = Movie::parse(stage().bytes()).unwrap_or_else(|error| unreachable!("{error}"));
    let color = Rgba {
        red: 1,
        green: 2,
        blue: 3,
        alpha: 4,
    };
    let fill = FillStyle::LinearGradient {
        matrix: p4kx_gfx::Matrix::IDENTITY,
        gradient: Gradient {
            spread: 0,
            interpolation: 0,
            stops: vec![GradientStop { ratio: 0, color }; 16],
        },
    };
    let mut assets = Assets::default();
    let shape = rect(6, (100, 100), TestFill::Solid(RED));
    let parsed = Movie::parse(
        stage()
            .tag(shape.code(), &shape.body())
            .tag(26, &TestPlace::new(1, 6).body2())
            .tag(26, &TestPlace::new(2, 6).at(500, 0).body2())
            .bytes(),
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    let parsed_assets = parsed.assets();
    let mut wide: Shape = parsed_assets.shapes[0].clone();
    wide.fill_styles = vec![fill];
    assets.shapes.push(wide);
    assets.timeline = parsed_assets.timeline;
    let scene = movie.first_frame(&assets);
    assert_eq!(
        scene
            .skipped
            .iter()
            .map(|skipped| skipped.reason)
            .collect::<Vec<_>>(),
        [SkipReason::GradientStops { count: 16 }; 2],
        "the gradient, for each placement"
    );
    assert!(scene.shapes[0].layers.is_empty(), "no layer");
    assert_eq!(
        scene.stage,
        Rect {
            x_min: 0,
            x_max: 2000,
            y_min: 0,
            y_max: 2000
        },
        "stage"
    );
}
