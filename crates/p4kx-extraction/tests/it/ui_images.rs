// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The textures and vector graphics that screens draw, read from an Archive by the path of a
//! Record.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{Archive, ArchiveImages, ConversionError};
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat};

type TestResult = Result<(), Box<dyn Error>>;

const HEADER: &str = "Data\\UI\\Textures\\Test\\quad.dds";
const SVG: &str = "Data/UI/Textures/Vector/Test/box.svg";
const BOX: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1 1\"/>";

fn write_archive(folder: &Path) -> Result<Archive, Box<dyn Error>> {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 32, 16).split(3);
    let path = folder.join("Data.p4k");
    dds.entries(HEADER)
        .into_iter()
        .chain([TestEntry::stored(SVG, BOX.to_vec())])
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

#[test]
fn a_texture_is_read_by_the_path_of_a_record_in_any_case() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let images = ArchiveImages::new(&archive);
    let image = images.texture("ui/textures/TEST/quad.dds", 1000)?;
    assert_eq!((image.width(), image.height()), (32, 16), "the largest mip");
    Ok(())
}

#[test]
fn the_smallest_mip_that_still_covers_the_screen_is_decoded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let images = ArchiveImages::new(&archive);
    let image = images.texture("UI/Textures/Test/quad.dds", 8)?;
    assert_eq!((image.width(), image.height()), (8, 4), "two levels down");
    let tiny = images.texture("UI/Textures/Test/quad.dds", 1)?;
    assert!(tiny.width() >= 1 && tiny.height() >= 1, "never empty");
    Ok(())
}

#[test]
fn a_texture_the_archive_lacks_is_an_unknown_entry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let result = ArchiveImages::new(&archive).texture("UI/Textures/Test/none.dds", 100);
    assert!(
        matches!(result, Err(ConversionError::UnknownEntry(_))),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_vector_graphic_is_read_up_to_its_limit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let images = ArchiveImages::new(&archive);
    let bytes = images.vector("UI/Textures/Vector/Test/BOX.svg", 1 << 20)?;
    assert_eq!(bytes, BOX, "the file");
    let refused = images.vector("UI/Textures/Vector/Test/box.svg", 8);
    assert!(
        matches!(refused, Err(ConversionError::ReadEntry { .. })),
        "{refused:?}"
    );
    let missing = images.vector("UI/Textures/Vector/Test/none.svg", 100);
    assert!(
        matches!(missing, Err(ConversionError::UnknownEntry(_))),
        "{missing:?}"
    );
    Ok(())
}
