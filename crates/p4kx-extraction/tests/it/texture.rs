// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Split DDS textures from an Archive and from files on disk.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{Archive, ConversionError, PathIndex, Texture};
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat};

type TestResult = Result<(), Box<dyn Error>>;

const HEADER: &str = "Data\\Textures\\rock_diff.dds";

fn texture() -> SplitDds {
    SplitDds::new(TestTextureFormat::BC7_SRGB, 32, 16).split(3)
}

fn standard_layout(dds: &SplitDds) -> Vec<u8> {
    let mut expected = dds
        .header_bytes()
        .get(..dds.header_len())
        .unwrap_or_default()
        .to_vec();
    for level in 0..6 {
        expected.extend_from_slice(&dds.mip(level));
    }
    expected
}

fn write_archive(folder: &Path, entries: Vec<TestEntry>) -> Result<Archive, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    entries
        .into_iter()
        .fold(
            ArchiveBuilder::new().entry(TestEntry::stored("Data\\other.dds.1", vec![0; 3])),
            ArchiveBuilder::entry,
        )
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

#[test]
fn a_texture_in_an_archive_merges_its_parts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = texture();
    let archive = write_archive(dir.path(), dds.entries(HEADER))?;

    let texture = Texture::from_archive(&archive, "data/textures/ROCK_DIFF.dds")?;
    let mut merged = Vec::new();
    let report = texture.merge_into(&mut merged)?;

    assert_eq!(texture.name(), HEADER, "named by its Entry path");
    assert_eq!(texture.header().mip_count(), 6, "header parsed");
    assert_eq!(texture.part_count()?, 3, "three parts");
    assert_eq!(report.parts, 3, "three parts merged");
    assert_eq!(merged, standard_layout(&dds), "standard DDS layout");
    Ok(())
}

#[test]
fn a_texture_on_disk_merges_into_a_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = texture();
    let header = dds.write_files(dir.path(), "rock_diff.dds")?;
    let out = dir.path().join("merged.dds");

    let report = Texture::from_file(&header)?.merge_to_file(&out)?;

    assert_eq!(std::fs::read(&out)?, standard_layout(&dds), "merged file");
    assert_eq!(report.size, std::fs::metadata(&out)?.len(), "reported size");
    Ok(())
}

#[test]
fn a_missing_part_in_an_archive_is_named_by_its_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut entries = texture().entries(HEADER);
    entries.remove(2);
    let archive = write_archive(dir.path(), entries)?;

    let result = Texture::from_archive(&archive, HEADER)?.merge_into(&mut std::io::sink());

    let Err(error @ ConversionError::MissingPart { .. }) = result else {
        panic!("expected a missing part, got {result:?}");
    };
    assert_eq!(
        error.to_string(),
        format!("{HEADER}.2 is missing"),
        "names the part"
    );
    Ok(())
}

#[test]
fn a_truncated_part_on_disk_is_named_by_its_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = texture().write_files(dir.path(), "rock_diff.dds")?;
    let part = dir.path().join("rock_diff.dds.3");
    std::fs::write(&part, [0; 10])?;
    let out = dir.path().join("merged.dds");

    let result = Texture::from_file(&header)?.merge_to_file(&out);

    let Err(error @ ConversionError::TruncatedPart { .. }) = result else {
        panic!("expected a truncated part, got {result:?}");
    };
    assert!(
        error.to_string().starts_with(&part.display().to_string()),
        "names the part: {error}"
    );
    assert!(!out.exists(), "nothing is written");
    Ok(())
}

#[test]
fn an_unknown_entry_is_reported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), texture().entries(HEADER))?;

    let result = Texture::from_archive(&archive, "Data\\missing.dds");

    assert!(
        matches!(result, Err(ConversionError::UnknownEntry(_))),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_texture_found_through_the_path_index_is_the_one_found_by_scanning() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = texture().alpha(TestTextureFormat::R8);
    let archive = write_archive(dir.path(), dds.entries(HEADER))?;
    let index = PathIndex::new(&archive);

    let indexed = Texture::from_index(&index, "data/textures/ROCK_DIFF.dds")?;
    let scanned = Texture::from_archive(&archive, HEADER)?;
    let (mut a, mut b) = (Vec::new(), Vec::new());
    indexed.merge_into(&mut a)?;
    scanned.merge_into(&mut b)?;

    assert_eq!(indexed.name(), HEADER, "named by its Entry path");
    assert_eq!(a, b, "the same parts");
    assert!(indexed.has_alpha_tail(), "the alpha tail is found");
    let tail = Texture::from_index(&index, "Data\\Textures\\rock_diff.dds.a")?;
    let mut tail_parts = Vec::new();
    tail.merge_into(&mut tail_parts)?;
    let scanned_tail = Texture::from_archive(&archive, "Data\\Textures\\rock_diff.dds.a")?;
    let mut scanned_parts = Vec::new();
    scanned_tail.merge_into(&mut scanned_parts)?;
    assert_eq!(
        tail_parts, scanned_parts,
        "a path ending in .a opens the tail"
    );
    assert_eq!(
        indexed.decode_mip(1)?,
        scanned.decode_mip(1)?,
        "decodes alike with the tail merged"
    );
    Ok(())
}

#[test]
fn an_unknown_entry_is_reported_through_the_path_index_too() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), texture().entries(HEADER))?;

    let result = Texture::from_index(&PathIndex::new(&archive), "Data\\missing.dds");

    assert!(
        matches!(result, Err(ConversionError::UnknownEntry(_))),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_missing_header_file_is_reported() {
    let result = Texture::from_file(Path::new("does-not-exist.dds"));

    assert!(
        matches!(result, Err(ConversionError::NoSuchFile { .. })),
        "{result:?}"
    );
}

#[test]
fn the_merge_never_overwrites_its_own_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = texture();
    let header = dds.write_files(dir.path(), "rock_diff.dds")?;

    for target in [header.clone(), dir.path().join("rock_diff.dds.2")] {
        let result = Texture::from_file(&header)?.merge_to_file(&target);

        assert!(
            matches!(result, Err(ConversionError::OutputIsInput { .. })),
            "{result:?}"
        );
    }
    assert_eq!(std::fs::read(&header)?, dds.header_bytes(), "header kept");
    Ok(())
}
