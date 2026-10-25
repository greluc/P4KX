// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The one image a texture's preview shows per mip, and which Entries are textures.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{Archive, Texture};
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestTextureFormat};

type TestResult = Result<(), Box<dyn Error>>;

const HEADER: &str = "Data\\Textures\\preview.dds";

fn write_archive(folder: &Path, dds: &SplitDds) -> Result<Archive, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    dds.entries(HEADER)
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> Option<&[u8]> {
    let start = usize::try_from((y * width + x) * 4).ok()?;
    pixels.get(start..start + 4)
}

#[test]
fn a_plain_texture_previews_its_mip() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = SplitDds::new(TestTextureFormat::LegacyBgra8, 16, 8).split(2);
    let archive = write_archive(dir.path(), &dds)?;
    let texture = Texture::from_archive(&archive, HEADER)?;

    for level in 0..texture.header().mip_count() {
        assert_eq!(
            texture.decode_preview(level)?,
            texture.decode_mip(level)?,
            "level {level}"
        );
    }
    Ok(())
}

#[test]
fn a_cubemap_previews_its_faces_as_a_cross() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = SplitDds::new(TestTextureFormat::LegacyBgra8, 4, 4)
        .cubemap()
        .split(2);
    let archive = write_archive(dir.path(), &dds)?;
    let texture = Texture::from_archive(&archive, HEADER)?;

    let cross = texture.decode_preview(1)?;
    let faces = texture.decode_surfaces(1)?;

    assert_eq!(
        (cross.width(), cross.height()),
        (8, 6),
        "four by three faces"
    );
    for (face, (column, row)) in faces
        .iter()
        .zip([(2, 1), (0, 1), (1, 0), (1, 2), (1, 1), (3, 1)])
    {
        assert_eq!(
            pixel(cross.pixels(), cross.width(), column * 2, row * 2),
            pixel(face.pixels(), face.width(), 0, 0),
            "face in cell {column}, {row}"
        );
    }
    Ok(())
}

#[test]
fn an_array_previews_its_elements_on_a_sheet() -> TestResult {
    let dir = tempfile::tempdir()?;
    let rgba8 = TestTextureFormat::Dx10Pixel {
        dxgi_format: 28,
        pixel_bytes: 4,
    };
    let dds = SplitDds::new(rgba8, 4, 2).array(3).split(2);
    let archive = write_archive(dir.path(), &dds)?;
    let texture = Texture::from_archive(&archive, HEADER)?;

    for level in 0..texture.header().mip_count() {
        let sheet = texture.decode_preview(level)?;
        let elements = texture.decode_surfaces(level)?;
        let (width, height) = (elements[0].width(), elements[0].height());

        assert_eq!(
            Some([sheet.width(), sheet.height()]),
            texture.header().preview_size(level),
            "the announced size at level {level}"
        );
        assert_eq!(
            (sheet.width(), sheet.height()),
            (width * 2, height * 2),
            "two by two cells at level {level}"
        );
        for (element, (column, row)) in elements.iter().zip([(0, 0), (1, 0), (0, 1)]) {
            assert_eq!(
                pixel(sheet.pixels(), sheet.width(), column * width, row * height),
                pixel(element.pixels(), width, 0, 0),
                "element in cell {column}, {row} at level {level}"
            );
        }
        assert_eq!(
            pixel(sheet.pixels(), sheet.width(), width, height),
            Some([0, 0, 0, 0].as_slice()),
            "the fourth cell is empty at level {level}"
        );
    }
    Ok(())
}

#[test]
fn a_volume_previews_its_slices_on_a_sheet() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = SplitDds::new(TestTextureFormat::LegacyBgra8, 4, 4)
        .volume(4)
        .split(1);
    let archive = write_archive(dir.path(), &dds)?;
    let texture = Texture::from_archive(&archive, HEADER)?;

    let sheet = texture.decode_preview(0)?;
    let slices = texture.decode_surfaces(0)?;

    assert_eq!((sheet.width(), sheet.height()), (8, 8), "two by two slices");
    assert_eq!(slices.len(), 4, "four slices");
    assert_eq!(
        pixel(sheet.pixels(), sheet.width(), 4, 4),
        pixel(slices[3].pixels(), 4, 0, 0),
        "the last slice is bottom right"
    );
    assert_eq!(
        texture.decode_preview(2)?.width(),
        1,
        "the smallest mip has one slice"
    );
    Ok(())
}

#[test]
fn texture_headers_and_alpha_tails_are_textures_but_their_parts_are_not() {
    for (path, header) in [
        ("Data\\a.dds", true),
        ("Data/A.DDS", true),
        ("Data\\a.dds.a", true),
        ("Data\\a.dds.A", true),
        ("Data\\a.dds.1", false),
        ("Data\\a.dds.3a", false),
        ("Data\\a.ddsx", false),
        ("Data\\a.txt", false),
        (".dds", false),
    ] {
        assert_eq!(Texture::is_header_path(path), header, "{path}");
    }
}
