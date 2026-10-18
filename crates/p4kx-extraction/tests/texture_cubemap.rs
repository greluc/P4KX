//! Cubemaps and alpha tails of split textures in an Archive and on disk.

use std::error::Error;
use std::fs::File;
use std::path::Path;

use p4kx_extraction::{
    Archive, CUBE_FACE_NAMES, ConversionError, CubemapLayout, Texture, TextureError,
};
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestTextureFormat};

type TestResult = Result<(), Box<dyn Error>>;

const HEADER: &str = "Data\\Textures\\probe_cm.dds";

fn write_archive(folder: &Path, dds: &SplitDds) -> Result<Archive, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    dds.entries(HEADER)
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(Archive::open(&path)?)
}

fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let mut reader = png::Decoder::new(std::io::BufReader::new(File::open(path)?)).read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut pixels)?;
    Ok((info.width, info.height, pixels))
}

fn cubemap() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyBgra8, 4, 4)
        .cubemap()
        .split(2)
}

/// Returns the first pixel of a BGRA8 image as RGBA.
const fn first_rgba(bgra: &[u8]) -> [u8; 4] {
    match bgra {
        [b, g, r, a, ..] => [*r, *g, *b, *a],
        _ => [0; 4],
    }
}

#[test]
fn a_split_cubemap_entry_merges_into_a_standard_cubemap() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = cubemap();
    let archive = write_archive(dir.path(), &dds)?;
    let mut merged = Vec::new();

    let report = Texture::from_archive(&archive, HEADER)?.merge_into(&mut merged)?;

    assert_eq!(
        merged,
        dds.merged_bytes(),
        "face by face, largest mip first"
    );
    assert_eq!(report.parts, 2, "two parts");
    assert!(report.header.is_cubemap(), "a cubemap");
    Ok(())
}

#[test]
fn a_cubemap_is_exported_as_six_face_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = cubemap();
    let archive = write_archive(dir.path(), &dds)?;
    let out = dir.path().join("probe.png");

    let report = Texture::from_archive(&archive, HEADER)?.png_to_file(&out, 0)?;

    assert!(!out.exists(), "no single file");
    assert_eq!(report.files.len(), 6, "six files");
    for ((face, name), file) in (0..).zip(CUBE_FACE_NAMES).zip(&report.files) {
        assert_eq!(
            file,
            &dir.path().join(format!("probe_{name}.png")),
            "face {face} is named {name}"
        );
        let (width, height, pixels) = read_png(file)?;
        assert_eq!((width, height), (4, 4), "face {face} is 4 x 4");
        assert_eq!(
            pixels.get(..4),
            Some(first_rgba(&dds.surface_mip(face, 0)).as_slice()),
            "face {face} holds its own pixels"
        );
    }
    Ok(())
}

#[test]
fn a_cubemap_is_exported_as_one_cross_on_request() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = cubemap();
    let header = dds.write_files(dir.path(), "probe_cm.dds")?;
    let out = dir.path().join("probe.png");

    let report = Texture::from_file(&header)?.png_to_file_as(&out, 1, CubemapLayout::Cross)?;

    assert_eq!(report.files, std::slice::from_ref(&out), "one file");
    assert_eq!(
        (report.width, report.height),
        (8, 6),
        "four by three faces of 2 x 2"
    );
    let (width, height, pixels) = read_png(&out)?;
    assert_eq!((width, height), (8, 6), "the cross");
    let row_len = 8 * 4;
    let pixel = |x: usize, y: usize| pixels.get(y * row_len + x * 4..y * row_len + x * 4 + 4);
    assert_eq!(
        pixel(2, 2),
        Some(first_rgba(&dds.surface_mip(4, 1)).as_slice()),
        "+Z starts the middle row's second cell"
    );
    assert_eq!(pixel(0, 0), Some([0, 0, 0, 0].as_slice()), "an empty cell");
    Ok(())
}

fn with_alpha() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 4)
        .split(2)
        .alpha(TestTextureFormat::R8)
}

#[test]
fn the_alpha_tail_of_an_entry_is_found_and_merged_into_alpha() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = with_alpha();
    let archive = write_archive(dir.path(), &dds)?;
    let texture = Texture::from_archive(&archive, HEADER)?;
    let tail = dds.alpha_tail().ok_or("an alpha tail")?;

    let found = texture.alpha_tail().ok_or("the alpha tail is found")??;
    assert_eq!(found.name(), format!("{HEADER}.a"), "the tail's Entry");
    assert_eq!(
        found.header().format().to_string(),
        "R8_UNORM",
        "its format"
    );
    assert_eq!(found.part_count()?, 2, "its parts");

    for level in [0, 3] {
        let image = texture.decode_mip(level)?;
        let colour = dds.surface_mip(0, level);
        let alpha = tail.surface_mip(0, level);
        let expected: Vec<u8> = colour
            .chunks(4)
            .zip(&alpha)
            .flat_map(|(bgra, alpha)| match bgra {
                [b, g, r, _] => [*r, *g, *b, *alpha],
                _ => [0; 4],
            })
            .collect();
        assert_eq!(
            image.pixels(),
            expected,
            "colour with the tail as alpha, mip {level}"
        );
    }
    Ok(())
}

#[test]
fn the_alpha_tail_of_a_file_is_found_and_reported_in_the_png_export() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = with_alpha().write_files(dir.path(), "glass.dds")?;
    let out = dir.path().join("glass.png");

    let report = Texture::from_file(&header)?.png_to_file(&out, 0)?;

    assert!(report.alpha_tail, "the alpha tail is merged");
    let tail = with_alpha().alpha_tail().ok_or("an alpha tail")?;
    let (_, _, pixels) = read_png(&out)?;
    assert_eq!(
        pixels.get(3),
        tail.surface_mip(0, 0).first(),
        "the first pixel's alpha"
    );
    Ok(())
}

#[test]
fn an_alpha_tail_opens_and_merges_on_its_own() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = with_alpha();
    let archive = write_archive(dir.path(), &dds)?;
    let tail = dds.alpha_tail().ok_or("an alpha tail")?;
    let mut merged = Vec::new();

    let texture = Texture::from_archive(&archive, &format!("{HEADER}.a"))?;
    let report = texture.merge_into(&mut merged)?;

    assert!(texture.alpha_tail().is_none(), "no tail of the tail");
    assert_eq!(report.parts, 2, "parts .1a and .2a");
    assert_eq!(
        merged,
        tail.merged_bytes(),
        "the tail as one DDS without magic"
    );
    Ok(())
}

#[test]
fn a_cubemap_with_an_alpha_tail_gets_alpha_on_every_face() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = cubemap().alpha(TestTextureFormat::R8);
    let header = dds.write_files(dir.path(), "probe_cm.dds")?;
    let tail = dds.alpha_tail().ok_or("an alpha tail")?;

    let faces = Texture::from_file(&header)?.decode_surfaces(0)?;

    for (face, image) in (0..).zip(&faces) {
        assert_eq!(
            image.pixels().get(3),
            tail.surface_mip(face, 0).first(),
            "the alpha of face {face}"
        );
    }
    Ok(())
}

#[test]
fn an_alpha_tail_of_another_size_is_named() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = with_alpha().write_files(dir.path(), "glass.dds")?;
    let small = SplitDds::new(TestTextureFormat::LegacyBgra8, 4, 4)
        .alpha(TestTextureFormat::R8)
        .alpha_tail()
        .ok_or("an alpha tail")?;
    std::fs::write(dir.path().join("glass.dds.a"), small.header_bytes())?;
    for index in 1..=2 {
        std::fs::remove_file(dir.path().join(format!("glass.dds.{index}a")))?;
    }

    let result = Texture::from_file(&header)?.decode_mip(2);

    assert!(
        matches!(
            &result,
            Err(ConversionError::Texture {
                source: TextureError::AlphaTailSize {
                    width: 1,
                    height: 1,
                    expected_width: 2,
                    expected_height: 1
                },
                ..
            })
        ),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn an_alpha_tail_with_fewer_mips_fails_the_missing_mip_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = with_alpha().write_files(dir.path(), "glass.dds")?;
    let short = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 4)
        .mips(2)
        .alpha(TestTextureFormat::R8)
        .alpha_tail()
        .ok_or("an alpha tail")?;
    std::fs::write(dir.path().join("glass.dds.a"), short.header_bytes())?;
    for index in 1..=2 {
        std::fs::remove_file(dir.path().join(format!("glass.dds.{index}a")))?;
    }
    let texture = Texture::from_file(&header)?;

    let present = texture.decode_mip(1)?;
    let missing = texture.decode_mip(3);

    assert_eq!(
        present.pixels().get(3),
        short.surface_mip(0, 1).first(),
        "mip 1 still gets its alpha"
    );
    let Err(ConversionError::Texture { texture, source }) = &missing else {
        panic!("mip 3 has no alpha: {missing:?}");
    };
    assert!(
        texture.ends_with("glass.dds.a"),
        "the tail is named: {texture}"
    );
    assert!(
        matches!(
            source,
            TextureError::AlphaTailMips {
                level: 3,
                mip_count: 2
            }
        ),
        "{source:?}"
    );
    Ok(())
}

#[test]
fn a_broken_alpha_tail_only_fails_where_it_is_used() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = with_alpha().write_files(dir.path(), "glass.dds")?;
    std::fs::write(dir.path().join("glass.dds.a"), b"not a texture")?;
    let texture = Texture::from_file(&header)?;

    let tail = texture.alpha_tail();
    let merged = texture.merge_to_file(&dir.path().join("merged.dds"));
    let png = texture.png_to_file(&dir.path().join("glass.png"), 0);

    assert!(texture.has_alpha_tail(), "the tail is there");
    assert!(matches!(tail, Some(Err(_))), "but broken: {tail:?}");
    assert!(merged.is_ok(), "the colour still merges: {merged:?}");
    let Err(ConversionError::Texture { texture, .. }) = &png else {
        panic!("the PNG needs the tail: {png:?}");
    };
    assert!(
        texture.ends_with("glass.dds.a"),
        "the tail is named: {texture}"
    );
    assert!(!dir.path().join("glass.png").exists(), "nothing is written");
    Ok(())
}

#[test]
fn writing_onto_an_alpha_part_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = with_alpha().write_files(dir.path(), "glass.dds")?;

    let result = Texture::from_file(&header)?.png_to_file(&dir.path().join("glass.dds.1a"), 0);

    assert!(
        matches!(result, Err(ConversionError::OutputIsInput { .. })),
        "{result:?}"
    );
    Ok(())
}
