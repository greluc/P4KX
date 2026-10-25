// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Loading the textures of a model's materials from an Archive.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{Archive, ArchiveTextures, PathIndex, TextureLoader};
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat};

type TestResult = Result<(), Box<dyn Error>>;

const HEADER: &str = "Data\\Textures\\glass_ddna.dds";
const PATH: &str = "textures/glass_ddna.dds";

fn archive_with_short_tail(folder: &Path) -> Result<Archive, Box<dyn Error>> {
    let colour = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 4);
    let tail = SplitDds::new(TestTextureFormat::R8, 8, 4)
        .mips(2)
        .alpha(TestTextureFormat::R8)
        .alpha_tail()
        .ok_or("an alpha tail")?;
    let path = folder.join("Data.p4k");
    colour
        .entries(HEADER)
        .into_iter()
        .chain([TestEntry::stored(
            format!("{HEADER}.a"),
            tail.header_bytes(),
        )])
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

#[test]
fn a_mip_beyond_the_alpha_tail_selects_the_last_mip_the_tail_has() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive_with_short_tail(dir.path())?;
    let index = PathIndex::new(&archive);
    let textures = ArchiveTextures::new(&index);

    let loaded = textures.load(PATH, 3)?;

    assert_eq!(
        (loaded.image.width(), loaded.image.height()),
        (4, 2),
        "mip 1, the last the tail has, not mip 3"
    );
    assert!(loaded.has_alpha, "the tail is in the alpha channel");
    Ok(())
}

#[test]
fn the_dimensions_are_known_without_decoding() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive_with_short_tail(dir.path())?;
    let index = PathIndex::new(&archive);
    let textures = ArchiveTextures::new(&index);

    assert_eq!(textures.dimensions(PATH, 0)?, Some((8, 4)), "mip 0");
    assert_eq!(
        textures.dimensions(PATH, 3)?,
        Some((4, 2)),
        "clamped to the tail's last mip"
    );
    assert!(
        textures.dimensions("textures/none.dds", 0).is_err(),
        "a missing texture"
    );
    Ok(())
}
