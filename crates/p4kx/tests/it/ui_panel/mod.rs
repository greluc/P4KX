//! The synthetic UI panel the `p4kx ui render` tests render: shapes with gradient, bitmap and
//! solid fills and strokes, a rotated and tinted sprite, a mask and a button the renderer skips.

pub(crate) mod reference;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{
    TestFill, TestGradient, TestMovie, TestPlace, TestShape, TestShapeRecord, TestSignature,
    background_body, lossless_body, sprite_body,
};

const AMBER: [u8; 4] = [255, 170, 0, 255];
const NAVY: [u8; 4] = [10, 20, 60, 255];
fn path_shape(
    id: u16,
    fill: TestFill,
    line: Option<(u16, [u8; 4])>,
    (start, edges): ((i32, i32), &[TestShapeRecord]),
) -> TestShape {
    let mut shape = TestShape::new(id, 3, [0, 1280, 0, 960]).fill(fill);
    if let Some((width, color)) = line {
        shape = shape.line(width, color);
    }
    let mut shape = shape.record(TestShapeRecord::Move {
        to: start,
        fill0: 0,
        fill1: 1,
        line: u32::from(line.is_some()),
    });
    for &edge in edges {
        shape = shape.record(edge);
    }
    shape
}

const fn rect_edges(width: i32, height: i32) -> [TestShapeRecord; 4] {
    [
        TestShapeRecord::Line(width, 0),
        TestShapeRecord::Line(0, height),
        TestShapeRecord::Line(-width, 0),
        TestShapeRecord::Line(0, -height),
    ]
}

/// The shapes of the panel: a gradient backdrop, a stroked amber dial of curves, a bitmap
/// badge, a bar and a stripe.
fn shapes() -> [TestShape; 5] {
    let backdrop = path_shape(
        1,
        TestFill::Gradient(
            NAVY,
            [40, 80, 160, 255],
            TestGradient::Linear { x: 0, width: 1280 },
        ),
        None,
        ((0, 0), &rect_edges(1280, 960)),
    );
    let dial = path_shape(
        2,
        TestFill::Gradient(
            AMBER,
            [120, 40, 0, 255],
            TestGradient::Radial {
                center: (200, 200),
                radius: 200,
                focal: 0,
            },
        ),
        Some((40, [255, 255, 255, 255])),
        (
            (200, 0),
            &[
                TestShapeRecord::Curve(200, 0, 0, 200),
                TestShapeRecord::Curve(0, 200, -200, 0),
                TestShapeRecord::Curve(-200, 0, 0, -200),
                TestShapeRecord::Curve(0, -200, 200, 0),
            ],
        ),
    );
    let badge = path_shape(
        3,
        TestFill::ScaledBitmap {
            id: 20,
            scale: 80,
            at: (0, 0),
            repeating: true,
            smoothed: false,
        },
        None,
        ((0, 0), &rect_edges(320, 160)),
    );
    let bar = path_shape(
        4,
        TestFill::Solid([0, 200, 120, 255]),
        None,
        ((0, 0), &rect_edges(400, 80)),
    );
    let stripe = path_shape(
        5,
        TestFill::Solid([255, 0, 0, 255]),
        None,
        ((0, 0), &rect_edges(1280, 200)),
    );
    [backdrop, dial, badge, bar, stripe]
}

/// A 64 x 48 pixel panel: the shapes, a sprite rotated and tinted, a mask cutting a stripe out
/// of an inverted backdrop, and a button the renderer skips.
pub(crate) fn panel() -> Vec<u8> {
    let checker = lossless_body(
        20,
        2,
        2,
        &[
            [255, 255, 255, 255],
            [0, 0, 0, 255],
            [0, 0, 0, 255],
            [255, 255, 255, 128],
        ],
        true,
    );
    let gauge = sprite_body(
        30,
        1,
        &[
            (26, TestPlace::new(1, 4).at(0, 0).body2()),
            (
                26,
                TestPlace::new(2, 4)
                    .at(0, 120)
                    .color([1.0, 0.5, 1.0, 1.0], [0, 0, 80, 0])
                    .body2(),
            ),
            (1, Vec::new()),
        ],
    );
    let mut movie = TestMovie::new(TestSignature::Cfx)
        .stage(64, 48)
        .tag(9, &background_body([5, 5, 5]))
        .tag(20, &checker)
        .tag(34, &[50, 0, 0])
        .tag(39, &gauge);
    for shape in shapes() {
        movie = movie.tag(shape.code(), &shape.body());
    }
    movie
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(26, &TestPlace::new(2, 2).at(100, 100).body2())
        .tag(26, &TestPlace::new(3, 3).at(800, 80).body2())
        .tag(
            26,
            &TestPlace::new(4, 30)
                .matrix([0.866, 0.866, 0.5, -0.5], (700, 400))
                .color([1.0, 1.0, 1.0, 0.75], [0, 0, 0, 0])
                .body2(),
        )
        .tag(26, &TestPlace::new(5, 5).at(0, 700).clip(6).body2())
        .tag(
            26,
            &TestPlace::new(6, 1)
                .color([-1.0, -1.0, -1.0, 1.0], [255, 255, 255, 0])
                .body2(),
        )
        .tag(26, &TestPlace::new(7, 50).body2())
        .tag(1, &[])
        .bytes()
}

pub(crate) fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

pub(crate) fn write_panel(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let file = dir.join("Panel.gfx");
    fs::write(&file, panel())?;
    Ok(file)
}
