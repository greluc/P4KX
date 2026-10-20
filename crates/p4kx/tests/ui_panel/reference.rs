//! Reference images of the UI render tests and their comparison.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

pub(crate) fn png_pixels(path: &Path) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
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
pub(crate) fn assert_matches_reference(rendered: &Path, name: &str) -> TestResult {
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
