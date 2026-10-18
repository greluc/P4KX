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
    Ok(Archive::open(&path)?)
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
