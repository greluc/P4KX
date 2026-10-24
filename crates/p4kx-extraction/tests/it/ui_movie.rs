// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! UI movies from an Archive or from disk: bitmaps written as PNG, external images decoded from
//! the DDS textures next to the movie, shapes written as JSON.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{Archive, ConversionError, Texture, UiAssetKind, UiExtraction, UiMovie};
use p4kx_testkit::{
    ArchiveBuilder, SplitDds, TestEntry, TestFill, TestMovie, TestShape, TestShapeRecord,
    TestSignature, TestTextureFormat, external_image_body, lossless_body, morph_shape_body,
    sub_image_body,
};

type TestResult = Result<(), Box<dyn Error>>;

const MOVIE: &str = "Data\\UI\\Screens\\assets\\SWF\\Panel.gfx";
const TEXTURE: &str = "Data\\UI\\Screens\\assets\\Textures\\panel_bg.dds";

fn texture() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 4)
}

fn movie() -> Vec<u8> {
    let shape = TestShape::new(4, 3, [0, 100, 0, 100])
        .fill(TestFill::Solid([1, 2, 3, 255]))
        .record(TestShapeRecord::Move {
            to: (0, 0),
            fill0: 1,
            fill1: 0,
            line: 0,
        })
        .record(TestShapeRecord::Line(100, 0));
    let line = TestShapeRecord::Line(10, 0);
    TestMovie::new(TestSignature::Cfx)
        .tag(
            36,
            &lossless_body(1, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 128]], true),
        )
        .tag(
            1009,
            &external_image_body(2, 2, 19, (8, 4), ("bg", "../Textures/panel_bg.tif")),
        )
        .tag(1008, &sub_image_body(3, 2, [2, 1, 6, 3]))
        .tag(
            1009,
            &external_image_body(5, 2, 14, (8, 4), ("gone", "../Textures/gone.dds")),
        )
        .tag(shape.code(), &shape.body())
        .tag(46, &morph_shape_body(6, ([1; 4], [2; 4]), &[line], &[line]))
        .bytes()
}

fn write_archive(folder: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let mut builder = ArchiveBuilder::new()
        .entry(TestEntry::zstd(MOVIE, movie()))
        .entry(TestEntry::stored(
            TEXTURE.replace(".dds", ".tif"),
            b"II*\0 not a DDS".to_vec(),
        ));
    for entry in texture().entries(TEXTURE) {
        builder = builder.entry(entry);
    }
    builder.write_to(&path)?;
    Ok(path)
}

fn png_pixels(path: &Path) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(path)?));
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().ok_or("too large")?];
    let frame = reader.next_frame(&mut pixels)?;
    pixels.truncate(frame.buffer_size());
    Ok((frame.width, frame.height, pixels))
}

fn check_extraction(extraction: &UiExtraction, folder: &Path, full: &[u8]) -> TestResult {
    let written: Vec<(UiAssetKind, u16)> = extraction
        .written
        .iter()
        .map(|written| (written.kind, written.id))
        .collect();
    assert_eq!(
        written,
        [
            (UiAssetKind::Bitmap, 1),
            (UiAssetKind::Bitmap, 2),
            (UiAssetKind::Bitmap, 3),
            (UiAssetKind::Shape, 4),
            (UiAssetKind::MorphShape, 6),
        ],
        "file order, kinds grouped"
    );
    let (width, height, lossless) = png_pixels(&folder.join("bitmap_1.png"))?;
    assert_eq!((width, height), (2, 1), "lossless size");
    assert_eq!(lossless, [255, 0, 0, 255, 0, 0, 255, 128], "straight alpha");
    let (width, height, external) = png_pixels(&folder.join("bitmap_2.png"))?;
    assert_eq!((width, height), (8, 4), "external size");
    assert_eq!(external, full, "the DDS texture");
    let (width, height, cut) = png_pixels(&folder.join("bitmap_3.png"))?;
    assert_eq!((width, height), (4, 2), "sub-image size");
    let row = |y: usize| {
        full.get((y * 8 + 2) * 4..(y * 8 + 6) * 4)
            .unwrap_or_default()
    };
    assert_eq!(cut, [row(1), row(2)].concat(), "cut from the texture");
    let json: serde_json::Value = serde_json::from_slice(&fs::read(folder.join("shape_4.json"))?)?;
    assert_eq!(
        json.pointer("/records/1/delta/x"),
        Some(&serde_json::Value::from(100)),
        "edge in twips"
    );
    assert!(folder.join("morph_shape_6.json").is_file(), "morph shape");
    assert_eq!(extraction.failures.len(), 1, "the missing texture");
    let failure = extraction.failures.first().ok_or("one failure")?;
    assert_eq!(
        (failure.kind, failure.id),
        (UiAssetKind::Bitmap, 5),
        "bitmap 5"
    );
    assert!(
        matches!(&failure.error, ConversionError::ExternalImageMissing { file, .. } if file == "../Textures/gone.dds"),
        "{}",
        failure.error
    );
    assert!(!folder.join("bitmap_5.png").exists(), "no file");
    Ok(())
}

#[test]
fn a_movie_entry_writes_its_bitmaps_external_images_and_shapes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    let movie = UiMovie::from_archive(&archive, "data/ui/screens/assets/swf/panel.GFX")?;
    assert_eq!(movie.name(), MOVIE, "the Entry path");
    let full = Texture::from_archive(&archive, TEXTURE)?.decode_mip(0)?;
    let out = dir.path().join("assets");
    let extraction = movie.extract_assets(&out)?;
    check_extraction(&extraction, &out, full.pixels())
}

#[test]
fn a_movie_file_finds_its_external_images_on_disk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let swf = dir.path().join("Data").join("assets").join("SWF");
    let textures = dir.path().join("Data").join("assets").join("Textures");
    fs::create_dir_all(&swf)?;
    fs::create_dir_all(&textures)?;
    fs::write(swf.join("Panel.gfx"), movie())?;
    let header = texture().write_files(&textures, "panel_bg.dds")?;
    let full = Texture::from_file(&header)?.decode_mip(0)?;
    let movie = UiMovie::from_file(&swf.join("Panel.gfx"))?;
    let out = dir.path().join("out");
    let extraction = movie.extract_assets(&out)?;
    check_extraction(&extraction, &out, full.pixels())
}

#[test]
fn missing_and_invalid_movies_are_errors() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    assert!(matches!(
        UiMovie::from_archive(&archive, "Data\\UI\\nothing.gfx"),
        Err(ConversionError::UnknownEntry(_))
    ));
    assert!(matches!(
        UiMovie::from_file(&dir.path().join("missing.gfx")),
        Err(ConversionError::NoSuchFile { .. })
    ));
    let text = dir.path().join("text.gfx");
    fs::write(&text, b"not a movie at all")?;
    assert!(matches!(
        UiMovie::from_file(&text),
        Err(ConversionError::UiMovie { .. })
    ));
    Ok(())
}

#[test]
fn an_external_image_on_disk_outside_the_data_folder_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let swf = dir.path().join("screens").join("SWF");
    fs::create_dir_all(&swf)?;
    fs::write(swf.join("Panel.gfx"), movie())?;
    let textures = dir.path().join("screens").join("Textures");
    fs::create_dir_all(&textures)?;
    texture().write_files(&textures, "panel_bg.dds")?;
    let movie = UiMovie::from_file(&swf.join("Panel.gfx"))?;
    let extraction = movie.extract_assets(&dir.path().join("out"))?;
    let refused: Vec<u16> = extraction
        .failures
        .iter()
        .filter(|failure| matches!(failure.error, ConversionError::ExternalImageOutside { .. }))
        .map(|failure| failure.id)
        .collect();
    assert_eq!(
        refused,
        [2, 5],
        "both external images leave the movie folder"
    );
    let cut = extraction.failures.iter().find(|failure| failure.id == 3);
    assert!(
        cut.is_some_and(|failure| matches!(
            failure.error,
            ConversionError::UiSubImage { image_id: 2, .. }
        )),
        "the sub-image of the refused image fails too"
    );
    Ok(())
}
