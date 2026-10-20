//! `p4kx ui render` end to end: rendered PNGs compared with reference images within a
//! tolerance, the size and background options, the report of what was left out, and bad
//! arguments.
//!
//! `P4KX_UPDATE_REFERENCES=1` rewrites the reference images from the current renderer.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, TestEntry, TestFill, TestGradient, TestMovie, TestPlace, TestShape,
    TestShapeRecord, TestSignature, background_body, lossless_body, sprite_body,
};
use predicates::prelude::predicate;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MOVIE: &str = "Data\\UI\\Screens\\SWF\\Panel.gfx";
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
/// of an inverted backdrop, and a text the renderer skips.
fn panel() -> Vec<u8> {
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
        .tag(11, &[50, 0, 0])
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

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

fn write_panel(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let file = dir.join("Panel.gfx");
    fs::write(&file, panel())?;
    Ok(file)
}

fn png_pixels(path: &Path) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(path)?));
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().ok_or("too large")?];
    let frame = reader.next_frame(&mut pixels)?;
    pixels.truncate(frame.buffer_size());
    Ok((frame.width, frame.height, pixels))
}

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("references")
        .join(name)
}

/// Compares a rendered PNG with its reference: same size, every channel within 4 for at least
/// 99.5 % of the pixels and within 48 for all of them.
fn assert_matches_reference(rendered: &Path, name: &str) -> TestResult {
    let expected_path = reference(name);
    if std::env::var_os("P4KX_UPDATE_REFERENCES").is_some() {
        fs::copy(rendered, &expected_path)?;
    }
    let (width, height, actual) = png_pixels(rendered)?;
    let (expected_width, expected_height, expected) = png_pixels(&expected_path)?;
    assert_eq!(
        (width, height),
        (expected_width, expected_height),
        "{name}: size"
    );
    let mut off = 0_usize;
    let mut worst = 0_u8;
    for (actual, expected) in actual
        .as_chunks::<4>()
        .0
        .iter()
        .zip(expected.as_chunks::<4>().0)
    {
        let diff = actual
            .iter()
            .zip(expected)
            .map(|(actual, expected)| actual.abs_diff(*expected))
            .max()
            .unwrap_or_default();
        worst = worst.max(diff);
        off += usize::from(diff > 4);
    }
    let pixels = (width * height) as usize;
    assert!(
        off * 200 <= pixels,
        "{name}: {off} of {pixels} pixels differ by more than 4"
    );
    assert!(worst <= 48, "{name}: a pixel differs by {worst}");
    Ok(())
}

#[test]
fn a_panel_renders_like_its_reference_at_stage_size() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    let out = dir.path().join("panel.png");
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("64 x 48 px; 1 elements left out"))
        .stderr(predicate::str::contains(
            "skipped: definition 50 at depth 7: DefineText definitions are not drawn",
        ));
    assert_matches_reference(&out, "panel.png")
}

#[test]
fn size_and_background_scale_the_panel_and_fill_under_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    let fitted = dir.path().join("fitted.png");
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .arg("--out")
        .arg(&fitted)
        .args(["--size", "160x48", "--background", "#20304080"])
        .assert()
        .code(1);
    assert_matches_reference(&fitted, "panel_fitted.png")?;
    let wide = dir.path().join("wide.png");
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .arg("--out")
        .arg(&wide)
        .args(["--size", "128", "--background", "movie"])
        .assert()
        .code(1);
    assert_matches_reference(&wide, "panel_wide.png")
}

#[test]
fn the_json_report_lists_what_was_left_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(MOVIE, panel()))
        .write_to(&archive)?;
    let out = dir.path().join("panel.png");
    let output = p4kx()?
        .args(["--json", "ui", "render"])
        .arg(&archive)
        .arg("data/ui/screens/swf/panel.gfx")
        .arg("--out")
        .arg(&out)
        .output()?;
    assert_eq!(output.status.code(), Some(1), "a text: {output:?}");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["movie"], MOVIE, "the Entry");
    assert_eq!(
        (report["width"].as_u64(), report["height"].as_u64()),
        (Some(64), Some(48)),
        "size"
    );
    assert_eq!(
        report["skipped"],
        serde_json::json!([{
            "id": 50,
            "depth": 7,
            "reason": "not_drawn",
            "drawn": false,
            "message": "DefineText definitions are not drawn",
        }]),
        "the text"
    );
    assert_eq!(report["failed"], serde_json::json!([]), "nothing failed");
    let mut redacted = report;
    assert_eq!(
        redacted["target"].as_str(),
        Some(out.display().to_string().as_str()),
        "the PNG"
    );
    redacted["target"] = Value::from("[DIR]/panel.png");
    insta::assert_snapshot!(serde_json::to_string_pretty(&redacted)?);
    assert_matches_reference(&out, "panel.png")
}

#[test]
fn bad_sizes_backgrounds_and_outputs_are_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    for (option, value) in [
        ("--size", "0x10"),
        ("--size", "20000"),
        ("--size", "8000x8000"),
        ("--size", "wide"),
        ("--background", "red"),
        ("--background", "#12345"),
    ] {
        p4kx()?
            .args(["ui", "render"])
            .arg(&file)
            .arg("--out")
            .arg(dir.path().join("x.png"))
            .arg(option)
            .arg(value)
            .assert()
            .code(2);
    }
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .arg("--out")
        .arg(&file)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is one of the inputs"));
    assert_eq!(fs::read(&file)?, panel(), "the movie is untouched");
    Ok(())
}
