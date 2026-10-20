//! Decoding the real Archive's high-range BC6H textures and exporting one as OpenEXR.

use std::io::Write;

use p4kx_extraction::{Archive, ConversionError, HdrImage, Texture, TextureError};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::TestResult;

const DECODE_LIMIT: usize = 40;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_real_bc6h_textures_decode_to_finite_high_range_values() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let high_range: Vec<Texture<'_>> = archive
        .entries()
        .par_iter()
        .filter(|entry| {
            entry
                .path()
                .as_bytes()
                .last_chunk::<4>()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(b".dds"))
        })
        .filter_map(|entry| Texture::from_archive(&archive, entry.path()).ok())
        .filter(|texture| texture.header().format().is_high_range())
        .collect();
    assert!(!high_range.is_empty(), "the Archive has BC6H textures");
    let stride = (high_range.len() / DECODE_LIMIT).max(1);

    let outcomes: Vec<Result<&str, String>> =
        high_range.par_iter().step_by(stride).map(decode).collect();

    let decoded = outcomes
        .iter()
        .filter(|outcome| outcome == &&Ok("decoded"))
        .count();
    let failures: Vec<&String> = outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().err())
        .collect();
    writeln!(
        std::io::stderr(),
        "{} BC6H textures, {} sampled, {decoded} decoded, the others have several images per mip",
        high_range.len(),
        outcomes.len()
    )?;
    assert!(failures.is_empty(), "decoding failed: {failures:#?}");
    export_one_exr(&high_range)
}

fn decode(texture: &Texture<'_>) -> Result<&'static str, String> {
    let header = texture.header();
    let level = header.mip_count().saturating_sub(1).min(2);
    match texture.decode_mip_hdr(level) {
        Ok(image) => {
            let expected = (
                (header.width() >> level).max(1),
                (header.height() >> level).max(1),
            );
            if (image.width(), image.height()) != expected {
                return Err(format!("{}: decoded {:?}", texture.name(), expected));
            }
            check_values(texture, &image)?;
            Ok("decoded")
        }
        Err(ConversionError::Texture {
            source: TextureError::UnsupportedImage { .. },
            ..
        }) => Ok("several images"),
        Err(error) => Err(format!("{}: {error}", texture.name())),
    }
}

fn check_values(texture: &Texture<'_>, image: &HdrImage) -> Result<(), String> {
    let signed = texture.header().format().to_string() == "BC6H_SF16";
    let bad = image
        .pixels()
        .iter()
        .find(|value| value.is_nan() || (!signed && **value < 0.0) || value.abs() > 65504.0);
    bad.map_or(Ok(()), |value| {
        Err(format!("{}: value {value}", texture.name()))
    })
}

fn export_one_exr(high_range: &[Texture<'_>]) -> TestResult {
    let Some(texture) = high_range
        .iter()
        .find(|texture| texture.header().surfaces() == 1 && texture.header().depth() == 1)
    else {
        writeln!(
            std::io::stderr(),
            "every BC6H texture has several images per mip"
        )?;
        return Ok(());
    };
    let target = tempfile::tempdir()?;
    let out = target.path().join("high_range.exr");

    let report = texture.exr_to_file(&out, 0)?;

    assert_eq!(
        (report.width, report.height),
        (texture.header().width(), texture.header().height()),
        "{}: the largest mip",
        texture.name()
    );
    let bytes = std::fs::read(&out)?;
    assert_eq!(
        bytes.get(..4),
        Some(&[0x76, 0x2F, 0x31, 0x01][..]),
        "an OpenEXR file"
    );
    Ok(())
}
