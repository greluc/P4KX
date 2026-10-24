// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;

use p4kx_gfx::{CanvasSize, RasterOptions, RgbaImage};
use p4kx_testkit::{DataForgeBuilder, TestRecord, entity_schema, screens_schema};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

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
