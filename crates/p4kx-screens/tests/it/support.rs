// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;

use p4kx_dataforge::DataForge;
use p4kx_gfx::{CanvasSize, RasterOptions, RgbaImage};
use p4kx_screens::{Composed, Limits, ScreenComposer};
use p4kx_testkit::{
    DataForgeBuilder, TestCanvas, TestColour, TestRecord, entity_schema, screens_schema,
};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const SCREEN: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0010;
pub(crate) const RED: TestColour = TestColour::Solid([255, 0, 0, 255]);
pub(crate) const GREEN: TestColour = TestColour::Solid([0, 255, 0, 255]);
pub(crate) const BLUE: TestColour = TestColour::Solid([0, 0, 255, 255]);
pub(crate) const NOTHING: [u8; 4] = [0, 0, 0, 0];

/// Composes the canvas `SCREEN` of `canvas`, with `others` in the same database.
pub(crate) fn compose_with(
    canvas: &TestCanvas,
    others: Vec<TestRecord>,
    limits: Limits,
) -> Result<Composed, Box<dyn Error>> {
    let bytes = database(std::iter::once(canvas.record()).chain(others));
    let forge = DataForge::open(&bytes)?;
    Ok(ScreenComposer::new(&forge)
        .with_limits(limits)
        .compose(&guid_text(SCREEN), None)?)
}

pub(crate) fn compose(canvas: &TestCanvas) -> Result<Composed, Box<dyn Error>> {
    compose_with(canvas, Vec::new(), Limits::DEFAULT)
}

pub(crate) fn render(canvas: &TestCanvas) -> Result<RgbaImage, Box<dyn Error>> {
    Ok(compose(canvas)?.rasterize(&stage())?.image)
}

pub(crate) fn render_with(
    canvas: &TestCanvas,
    others: Vec<TestRecord>,
) -> Result<RgbaImage, Box<dyn Error>> {
    Ok(compose_with(canvas, others, Limits::DEFAULT)?
        .rasterize(&stage())?
        .image)
}

fn decode(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or("the reference image is too large")?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels)?;
    pixels.truncate(info.buffer_size());
    Ok((info.width, info.height, pixels))
}

/// Compares `image` with the PNG `reference` below the folder of the crate, allowing `tolerance`
/// per channel on at most `most` pixels; `P4KX_UPDATE_REFERENCE` writes the reference again.
pub(crate) fn matches_reference(
    reference: &str,
    image: &RgbaImage,
    tolerance: u8,
    most: usize,
) -> TestResult {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(reference);
    let mut written = Vec::new();
    image.write_png(&mut written)?;
    if std::env::var_os("P4KX_UPDATE_REFERENCE").is_some() {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(&path, &written)?;
    }
    let (width, height, expected) = decode(&std::fs::read(&path)?)?;
    let (actual_width, actual_height, actual) = decode(&written)?;
    assert_eq!((actual_width, actual_height), (width, height), "the size");
    let differing = expected
        .chunks(4)
        .zip(actual.chunks(4))
        .filter(|(expected, actual)| {
            expected
                .iter()
                .zip(*actual)
                .any(|(expected, actual)| expected.abs_diff(*actual) > tolerance)
        })
        .count();
    assert!(
        differing <= most,
        "{differing} of {} pixels differ from {reference}",
        width * height
    );
    Ok(())
}

pub(crate) fn red_at(image: &RgbaImage, x: u32, y: u32) -> bool {
    near(pixel(image, x, y), [255, 0, 0, 255], 1)
}

pub(crate) fn green_at(image: &RgbaImage, x: u32, y: u32) -> bool {
    near(pixel(image, x, y), [0, 255, 0, 255], 1)
}

pub(crate) fn blue_at(image: &RgbaImage, x: u32, y: u32) -> bool {
    near(pixel(image, x, y), [0, 0, 255, 255], 1)
}

pub(crate) fn empty_at(image: &RgbaImage, x: u32, y: u32) -> bool {
    pixel(image, x, y) == NOTHING
}

pub(crate) fn database(records: impl IntoIterator<Item = TestRecord>) -> Vec<u8> {
    records
        .into_iter()
        .fold(
            screens_schema(entity_schema(DataForgeBuilder::new())),
            DataForgeBuilder::record,
        )
        .build()
}

pub(crate) fn guid_text(guid: u128) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        guid >> 96,
        (guid >> 80) & 0xffff,
        (guid >> 64) & 0xffff,
        (guid >> 48) & 0xffff,
        guid & 0xffff_ffff_ffff
    )
}

pub(crate) const fn stage() -> RasterOptions {
    RasterOptions {
        size: CanvasSize::Stage,
        background: p4kx_gfx::Background::Transparent,
    }
}

pub(crate) fn pixel(image: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let at = (y as usize * image.width() as usize + x as usize) * 4;
    image
        .pixels()
        .get(at..at + 4)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .unwrap_or_default()
}

pub(crate) fn near(actual: [u8; 4], expected: [u8; 4], tolerance: u8) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(actual, expected)| actual.abs_diff(expected) <= tolerance)
}
