// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Exporting a high-range BC6H texture as OpenEXR, and as tone-mapped PNG on request.

use std::error::Error;
use std::path::Path;

use exr::prelude::{ReadChannels, ReadLayers, Vec2};
use p4kx_extraction::{
    CUBE_FACE_NAMES, ConversionError, CubemapLayout, DxgiFormat, ExrReport, Texture, TextureError,
    TextureFormat,
};
use p4kx_testkit::{SplitDds, TestTextureFormat};
use p4kx_texture::decode_hdr;

type TestResult = Result<(), Box<dyn Error>>;

const fn bc6h(dxgi_format: u32) -> TestTextureFormat {
    TestTextureFormat::Dx10Block {
        dxgi_format,
        block_bytes: 16,
    }
}

fn read_exr(path: &Path) -> Result<(Vec2<usize>, Vec<f32>), Box<dyn Error>> {
    let image = exr::prelude::read()
        .no_deep_data()
        .largest_resolution_level()
        .rgba_channels(
            |resolution, _| (resolution, vec![0.0_f32; resolution.area() * 3]),
            |(resolution, pixels): &mut (Vec2<usize>, Vec<f32>),
             position: Vec2<usize>,
             (r, g, b, _): (f32, f32, f32, f32)| {
                let start = (position.y() * resolution.width() + position.x()) * 3;
                if let Some(slot) = pixels.get_mut(start..start + 3) {
                    slot.copy_from_slice(&[r, g, b]);
                }
            },
        )
        .first_valid_layer()
        .all_attributes()
        .from_file(path)?;
    Ok(image.layer_data.channel_data.pixels)
}

#[test]
fn a_bc6h_mip_is_exported_as_exr_with_its_decoded_values() -> TestResult {
    for format in [95, 96] {
        let dir = tempfile::tempdir()?;
        let header = SplitDds::new(bc6h(format), 32, 16)
            .split(2)
            .write_files(dir.path(), "sky_cube.dds")?;
        let texture = Texture::from_file(&header)?;
        let out = dir.path().join("sky.exr");

        let report = texture.exr_to_file(&out, 1)?;

        assert_eq!(
            report,
            ExrReport {
                level: 1,
                width: 16,
                height: 8,
                files: vec![out.clone()],
            },
            "the report of DXGI format {format}"
        );
        let (size, pixels) = read_exr(&out)?;
        assert_eq!((size.width(), size.height()), (16, 8), "mip 1 of 32×16");
        assert_eq!(
            pixels,
            texture.decode_mip_hdr(1)?.pixels(),
            "the decoded values of DXGI format {format}"
        );
    }
    Ok(())
}

#[test]
fn a_bc6h_mip_is_exported_as_tone_mapped_png_on_request() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = SplitDds::new(bc6h(95), 16, 16).write_files(dir.path(), "sky_cube.dds")?;
    let texture = Texture::from_file(&header)?;
    let out = dir.path().join("sky.png");

    let report = texture.png_to_file(&out, 0)?;

    assert_eq!((report.width, report.height), (16, 16), "the largest mip");
    assert!(out.exists(), "the PNG is written");
    Ok(())
}

#[test]
fn a_texture_without_high_range_is_not_exported_as_exr() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header =
        SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 16).write_files(dir.path(), "a.dds")?;
    let texture = Texture::from_file(&header)?;
    let out = dir.path().join("a.exr");

    let result = texture.exr_to_file(&out, 0);

    assert!(
        matches!(
            &result,
            Err(ConversionError::Texture {
                source: TextureError::NotHighRange(format),
                ..
            }) if format.to_string() == "BC1_UNORM"
        ),
        "{result:?}"
    );
    assert!(!out.exists(), "nothing is written");
    Ok(())
}

#[test]
fn an_exr_onto_an_input_or_of_a_missing_mip_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = SplitDds::new(bc6h(95), 16, 16)
        .split(1)
        .write_files(dir.path(), "sky_cube.dds")?;
    let texture = Texture::from_file(&header)?;

    let onto_part = texture.exr_to_file(&dir.path().join("sky_cube.dds.1"), 0);
    let beyond = texture.exr_to_file(&dir.path().join("sky.exr"), 9);

    assert!(
        matches!(onto_part, Err(ConversionError::OutputIsInput { .. })),
        "{onto_part:?}"
    );
    assert!(
        matches!(
            beyond,
            Err(ConversionError::Texture {
                source: TextureError::NoSuchMip { level: 9, .. },
                ..
            })
        ),
        "{beyond:?}"
    );
    Ok(())
}

#[test]
fn a_bc6h_cubemap_is_exported_as_six_exr_faces_in_dds_order() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = SplitDds::new(bc6h(96), 8, 8).cubemap().split(2);
    let header = dds.write_files(dir.path(), "sky_cube.dds")?;
    let texture = Texture::from_file(&header)?;

    let report = texture.exr_to_file(&dir.path().join("sky.exr"), 0)?;

    assert_eq!(report.files.len(), 6, "six faces");
    for ((face, file), name) in (0..).zip(&report.files).zip(CUBE_FACE_NAMES) {
        assert_eq!(
            file,
            &dir.path().join(format!("sky_{name}.exr")),
            "face {face} is named {name}"
        );
        let expected = decode_hdr(
            TextureFormat::Dxgi(DxgiFormat(96)),
            8,
            8,
            &dds.surface_mip(face, 0),
        )?;
        let (size, pixels) = read_exr(file)?;
        assert_eq!((size.width(), size.height()), (8, 8), "face {face} size");
        assert_eq!(pixels, expected.pixels(), "face {face} holds its own data");
    }
    Ok(())
}

#[test]
fn a_bc6h_cubemap_is_exported_as_one_exr_cross() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = SplitDds::new(bc6h(95), 4, 4).cubemap().split(1);
    let header = dds.write_files(dir.path(), "sky_cube.dds")?;
    let texture = Texture::from_file(&header)?;
    let out = dir.path().join("sky.exr");

    let report = texture.exr_to_file_as(&out, 0, CubemapLayout::Cross)?;

    assert_eq!(report.files, vec![out.clone()], "one file");
    let (size, pixels) = read_exr(&out)?;
    assert_eq!(
        (size.width(), size.height()),
        (16, 12),
        "a cross of 4 × 3 faces"
    );
    let faces = texture.decode_surfaces_hdr(0)?;
    let positive_z = faces.get(4).map(|face| face.pixels().to_vec());
    let centre: Vec<f32> = (4..8)
        .flat_map(|row| pixels[(row * 16 + 4) * 3..(row * 16 + 8) * 3].to_vec())
        .collect();
    assert_eq!(Some(centre), positive_z, "+Z sits in the centre");
    Ok(())
}
