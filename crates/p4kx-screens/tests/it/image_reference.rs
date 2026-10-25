// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The synthetic image canvas against a stored reference image.
//!
//! Set `P4KX_UPDATE_REFERENCE` to write the reference again after a deliberate change.

use std::io::Cursor;
use std::path::PathBuf;

use crate::images::composed;
use crate::support::{TestResult, stage};

const REFERENCE: &str = "tests/it/reference/image_sample.png";
const TOLERANCE: u8 = 3;
const MAX_DIFFERING_PIXELS: usize = 12;

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REFERENCE)
}

fn decode(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), Box<dyn std::error::Error>> {
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or("the reference image is too large")?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels)?;
    pixels.truncate(info.buffer_size());
    Ok((info.width, info.height, pixels))
}

#[test]
fn the_image_canvas_matches_its_reference_image() -> TestResult {
    let raster = composed()?.rasterize(&stage())?;
    let mut written = Vec::new();
    raster.image.write_png(&mut written)?;
    if std::env::var_os("P4KX_UPDATE_REFERENCE").is_some() {
        if let Some(folder) = path().parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(path(), &written)?;
    }
    let reference = std::fs::read(path())?;
    let (width, height, expected) = decode(&reference)?;
    let (actual_width, actual_height, actual) = decode(&written)?;
    assert_eq!((actual_width, actual_height), (width, height), "the size");
    let differing = expected
        .chunks(4)
        .zip(actual.chunks(4))
        .filter(|(expected, actual)| {
            expected
                .iter()
                .zip(*actual)
                .any(|(expected, actual)| expected.abs_diff(*actual) > TOLERANCE)
        })
        .count();
    assert!(
        differing <= MAX_DIFFERING_PIXELS,
        "{differing} of {} pixels differ from {REFERENCE}",
        width * height
    );
    Ok(())
}
