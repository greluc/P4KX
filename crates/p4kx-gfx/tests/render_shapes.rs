//! Rendering shapes: solid, gradient and bitmap fills, strokes, canvas sizes and backgrounds.

mod support;

use std::collections::HashMap;

use p4kx_gfx::{Background, CanvasSize, GfxError, RasterOptions, Rgba};
use p4kx_testkit::{
    TestFill, TestGradient, TestPlace, TestShape, TestShapeRecord, background_body, lossless_body,
};
use support::{BLUE, CLEAR, RED, assert_pixel, pixel, rect, render, scene, stage};

fn red_square() -> Vec<u8> {
    let square = rect(4, (1000, 1000), TestFill::Solid(RED));
    stage()
        .tag(square.code(), &square.body())
        .tag(26, &TestPlace::new(1, 4).at(200, 200).body2())
        .tag(1, &[])
        .bytes()
}

#[test]
fn a_solid_shape_is_drawn_where_it_is_placed() {
    let raster = render(&red_square(), CanvasSize::Stage);
    let image = &raster.image;
    assert_eq!((image.width(), image.height()), (100, 100), "stage size");
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
    assert_pixel(image, (30, 30), RED, 0);
    assert_pixel(image, (10, 59), RED, 0);
    assert_pixel(image, (5, 5), CLEAR, 0);
    assert_pixel(image, (61, 30), CLEAR, 0);
}

#[test]
fn the_size_scales_the_stage_and_fits_it_centred() {
    let raster = render(&red_square(), CanvasSize::Width(200));
    assert_eq!(
        (raster.image.width(), raster.image.height()),
        (200, 200),
        "keeps the aspect ratio"
    );
    assert_pixel(&raster.image, (21, 21), RED, 0);
    assert_pixel(&raster.image, (119, 119), RED, 0);
    assert_pixel(&raster.image, (121, 60), CLEAR, 0);
    let fit = render(
        &red_square(),
        CanvasSize::Fit {
            width: 200,
            height: 100,
        },
    );
    assert_eq!(
        (fit.image.width(), fit.image.height()),
        (200, 100),
        "exact size"
    );
    assert_pixel(&fit.image, (61, 11), RED, 0);
    assert_pixel(&fit.image, (55, 30), CLEAR, 0);
    assert_pixel(&fit.image, (109, 59), RED, 0);
}

#[test]
fn a_background_colour_lies_under_the_frame() {
    let (_, scene) = scene(&red_square());
    let options = RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Color(Rgba {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        }),
    };
    let raster = scene.rasterize(&HashMap::new(), &options);
    let image = raster.map(|raster| raster.image).ok();
    let image = image.as_ref().unwrap_or_else(|| unreachable!("renders"));
    assert_pixel(image, (5, 5), BLUE, 0);
    assert_pixel(image, (30, 30), RED, 0);
}

#[test]
fn the_movie_background_is_used_when_asked_for() {
    let bytes = stage().tag(9, &background_body([10, 20, 30])).bytes();
    let (_, scene) = scene(&bytes);
    let render = |background| {
        let options = RasterOptions {
            size: CanvasSize::Width(10),
            background,
        };
        scene
            .rasterize(&HashMap::new(), &options)
            .map_or_else(|error| unreachable!("{error}"), |raster| raster.image)
    };
    assert_pixel(&render(Background::Movie), (5, 5), [10, 20, 30, 255], 0);
    assert_pixel(&render(Background::Transparent), (5, 5), CLEAR, 0);
}

#[test]
fn linear_and_radial_gradients_run_between_their_colours() {
    let linear = rect(
        1,
        (2000, 1000),
        TestFill::Gradient(RED, BLUE, TestGradient::Linear { x: 0, width: 2000 }),
    );
    let radial = rect(
        2,
        (2000, 1000),
        TestFill::Gradient(
            RED,
            BLUE,
            TestGradient::Radial {
                center: (1000, 500),
                radius: 500,
                focal: 0,
            },
        ),
    );
    let bytes = stage()
        .tag(linear.code(), &linear.body())
        .tag(radial.code(), &radial.body())
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(26, &TestPlace::new(2, 2).at(0, 1000).body2())
        .tag(1, &[])
        .bytes();
    let image = render(&bytes, CanvasSize::Stage).image;
    assert_pixel(&image, (0, 25), RED, 8);
    assert_pixel(&image, (50, 25), [128, 0, 127, 255], 8);
    assert_pixel(&image, (99, 25), BLUE, 8);
    assert_pixel(&image, (50, 75), RED, 12);
    assert_pixel(&image, (75, 75), BLUE, 12);
    assert_pixel(&image, (62, 75), [128, 0, 127, 255], 16);
}

#[test]
fn a_focal_gradient_starts_at_its_focal_point() {
    let focal = rect(
        1,
        (2000, 2000),
        TestFill::Gradient(
            RED,
            BLUE,
            TestGradient::Radial {
                center: (1000, 1000),
                radius: 1000,
                focal: 192,
            },
        ),
    );
    let bytes = stage()
        .tag(focal.code(), &focal.body())
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(1, &[])
        .bytes();
    let image = render(&bytes, CanvasSize::Stage).image;
    assert_pixel(&image, (87, 50), RED, 24);
    assert_pixel(&image, (50, 50), [149, 0, 106, 255], 40);
}

fn quadrants(repeating: bool) -> Vec<u8> {
    smoothable_quadrants(repeating, false)
}

fn smoothable_quadrants(repeating: bool, smoothed: bool) -> Vec<u8> {
    let white = [255, 255, 255, 255];
    let green = [0, 255, 0, 255];
    let bitmap = lossless_body(7, 2, 2, &[RED, green, BLUE, white], false);
    let fill = TestFill::ScaledBitmap {
        id: 7,
        scale: 400,
        at: (0, 0),
        repeating,
        smoothed,
    };
    let square = rect(1, (1600, 800), fill);
    stage()
        .tag(20, &bitmap)
        .tag(square.code(), &square.body())
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(1, &[])
        .bytes()
}

#[test]
fn a_bitmap_fill_maps_its_pixels_and_repeats_or_clamps() {
    let raster = render(&quadrants(true), CanvasSize::Stage);
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
    let image = &raster.image;
    assert_pixel(image, (10, 10), RED, 0);
    assert_pixel(image, (30, 10), [0, 255, 0, 255], 0);
    assert_pixel(image, (10, 30), BLUE, 0);
    assert_pixel(image, (30, 30), [255, 255, 255, 255], 0);
    assert_pixel(image, (50, 10), RED, 0);
    let clamped = render(&quadrants(false), CanvasSize::Stage).image;
    assert_pixel(&clamped, (50, 10), [0, 255, 0, 255], 0);
    assert_pixel(&clamped, (70, 30), [255, 255, 255, 255], 0);
}

#[test]
fn a_smoothed_bitmap_blends_at_its_pixel_edges_and_a_plain_one_does_not() {
    let plain = render(&smoothable_quadrants(true, false), CanvasSize::Stage).image;
    assert_pixel(&plain, (19, 10), RED, 0);
    assert_pixel(&plain, (20, 10), [0, 255, 0, 255], 0);
    let smooth = render(&smoothable_quadrants(true, true), CanvasSize::Stage).image;
    let near_edge = pixel(&smooth, 19, 10);
    assert!(
        near_edge[0] > 60 && near_edge[0] < 240 && near_edge[1] > 20,
        "{near_edge:?} mixes red and green"
    );
    assert_pixel(&smooth, (10, 10), RED, 24);
}

#[test]
fn a_bitmap_fill_without_its_bitmap_is_skipped_and_listed() {
    let (_, scene) = scene(&quadrants(true));
    assert_eq!(scene.bitmap_ids(), [7], "named");
    let options = RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    };
    let raster = scene
        .rasterize(&HashMap::new(), &options)
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(raster.skipped.len(), 1, "{:?}", raster.skipped);
    assert_eq!(raster.skipped[0].id, 1, "the shape");
    assert_eq!(
        raster.skipped[0].reason.to_string(),
        "bitmap 7 could not be decoded",
        "reason"
    );
    assert_pixel(&raster.image, (10, 10), CLEAR, 0);
}

#[test]
fn a_line_style_strokes_the_edges() {
    let line = TestShape::new(1, 3, [0, 2000, 0, 2000])
        .line(100, [0, 0, 0, 255])
        .record(TestShapeRecord::Move {
            to: (200, 1000),
            fill0: 0,
            fill1: 0,
            line: 1,
        })
        .record(TestShapeRecord::Line(1600, 0))
        .record(TestShapeRecord::Curve(0, 800, -800, 0));
    let bytes = stage()
        .tag(line.code(), &line.body())
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(1, &[])
        .bytes();
    let image = render(&bytes, CanvasSize::Stage).image;
    assert_pixel(&image, (50, 50), [0, 0, 0, 255], 0);
    assert_pixel(&image, (50, 51), [0, 0, 0, 255], 0);
    assert_pixel(&image, (50, 52), [0, 0, 0, 128], 1);
    assert_pixel(&image, (50, 45), CLEAR, 0);
    assert_pixel(&image, (5, 50), CLEAR, 0);
    assert_pixel(&image, (60, 70), CLEAR, 0);
}

#[test]
fn canvases_beyond_the_limits_and_empty_stages_are_errors() {
    let (_, scene) = scene(&red_square());
    for size in [
        CanvasSize::Width(16_384),
        CanvasSize::Fit {
            width: 20_000,
            height: 10,
        },
        CanvasSize::Fit {
            width: 0,
            height: 10,
        },
    ] {
        let options = RasterOptions {
            size,
            background: Background::Transparent,
        };
        assert!(
            matches!(
                scene.rasterize(&HashMap::new(), &options),
                Err(GfxError::CanvasSize { .. })
            ),
            "{size:?}"
        );
    }
    let (_, empty) = support::scene(&stage().stage(0, 0).bytes());
    let options = RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    };
    assert!(
        matches!(
            empty.rasterize(&HashMap::new(), &options),
            Err(GfxError::EmptyStage)
        ),
        "no area"
    );
}
