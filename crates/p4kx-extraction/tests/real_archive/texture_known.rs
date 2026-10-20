//! Known textures of the real Archive merged and exported into a folder under `TMP`.

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use p4kx_extraction::{
    Archive, CUBE_FACE_NAMES, ConversionError, CubemapLayout, Entry, Texture, UnknownEntries,
};
use p4kx_testkit::real_archive_path;

use crate::TestResult;

const ICONS: [(&str, u32, u32); 2] = [
    (
        "Data\\UI\\Textures\\EA\\VehicleIcons\\VehicleIcon_ANVL_Valkyrie.dds",
        1024,
        1024,
    ),
    (
        "Data\\UI\\Textures\\Mobiglas\\TradingApp\\PTA_Icon_Failure.dds",
        256,
        256,
    ),
];
const CUBEMAPS: [&str; 2] = [
    "Data\\Textures\\cubemaps\\bhvr\\environmentprobeevening_cm.dds",
    "Engine\\EngineAssets\\Textures\\EnvProbes\\environmentprobe_stanton2c_sunny_cm.dds",
];
const CUBE_SIDE: u32 = 256;
const ALPHA_TAILS: [&str; 2] = [
    "Data\\Objects\\planets\\flora\\tree\\acacia_01\\acacia_01_b_bboard_ddna.dds",
    "Data\\Objects\\planets\\flora\\tree\\twigs_01\\twigs_01_ddna.dds",
];
const PROBE: &str = "Data\\Textures\\cubemaps\\inventory_setup\\cm_inventory_probe_cm.dds";
const CRYENGINE_TRAILER_LEN: u64 = 8;

fn open_first<'a>(archive: &'a Archive, paths: &[&str]) -> Result<Texture<'a>, ConversionError> {
    let mut missing = Vec::new();
    for path in paths {
        match Texture::from_archive(archive, path) {
            Err(ConversionError::UnknownEntry(UnknownEntries(unknown))) => missing.extend(unknown),
            found => return found,
        }
    }
    Err(ConversionError::UnknownEntry(UnknownEntries(missing)))
}

fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let mut reader = png::Decoder::new(BufReader::new(File::open(path)?)).read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut pixels)?;
    Ok((info.width, info.height, pixels))
}

fn varies(channel: usize, rgba: &[u8]) -> bool {
    let mut values = rgba.iter().skip(channel).step_by(4);
    values
        .next()
        .is_some_and(|first| values.any(|value| value != first))
}

fn merged_size(texture: &Texture<'_>) -> Result<u64, Box<dyn Error>> {
    let header = texture.header();
    let mips: u64 = (0..header.mip_count())
        .map(|level| header.mip_size(level).ok_or("a mip size"))
        .sum::<Result<u64, _>>()?;
    Ok(header.header_len() as u64 + mips)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_known_icon_decodes_to_its_dimensions() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let dir = tempfile::tempdir()?;
    let mut checked = 0;
    for (path, width, height) in ICONS {
        let texture = match Texture::from_archive(&archive, path) {
            Ok(texture) => texture,
            Err(ConversionError::UnknownEntry(_)) => continue,
            Err(error) => return Err(error.into()),
        };
        let image = texture.decode_mip(0)?;
        assert_eq!((image.width(), image.height()), (width, height), "{path}");
        assert!(varies(0, image.pixels()), "{path} is not one flat colour");

        let out = dir.path().join(format!("icon{checked}.png"));
        let report = texture.png_to_file(&out, 0)?;
        assert_eq!((report.width, report.height), (width, height), "{path}");
        let (png_width, png_height, pixels) = read_png(&out)?;
        assert_eq!((png_width, png_height), (width, height), "PNG of {path}");
        assert_eq!(pixels, image.into_pixels(), "PNG pixels of {path}");
        checked += 1;
    }
    assert!(checked > 0, "the Archive holds none of {ICONS:?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_known_cubemap_exports_as_faces_a_cross_and_exr() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let texture = open_first(&archive, &CUBEMAPS)?;
    let dir = tempfile::tempdir()?;
    assert!(texture.header().is_cubemap(), "a cubemap");
    assert!(texture.part_count()? > 0, "its larger mips are split off");

    let merged = dir.path().join("cube.dds");
    let report = texture.merge_to_file(&merged)?;
    assert_eq!(report.size, merged_size(&texture)?, "the merged size");
    assert_eq!(std::fs::metadata(&merged)?.len(), report.size, "the file");

    let faces = texture.png_to_file_as(&dir.path().join("cube.png"), 0, CubemapLayout::Faces)?;
    assert_eq!(faces.files.len(), 6, "one file per face");
    for (file, name) in faces.files.iter().zip(CUBE_FACE_NAMES) {
        assert!(
            file.to_string_lossy()
                .ends_with(&format!("cube_{name}.png"))
        );
        let (width, height, pixels) = read_png(file)?;
        assert_eq!((width, height), (CUBE_SIDE, CUBE_SIDE), "face {name}");
        assert!(varies(0, &pixels), "face {name} is not one flat colour");
    }

    let cross_file = dir.path().join("cross.png");
    texture.png_to_file_as(&cross_file, 0, CubemapLayout::Cross)?;
    let (width, height, _) = read_png(&cross_file)?;
    assert_eq!((width, height), (CUBE_SIDE * 4, CUBE_SIDE * 3), "the cross");

    let exr = texture.exr_to_file(&dir.path().join("cube.exr"), 0)?;
    assert_eq!(exr.files.len(), 6, "one EXR per face");
    assert_eq!((exr.width, exr.height), (CUBE_SIDE, CUBE_SIDE), "EXR faces");
    assert!(exr.files.iter().all(|file| file.is_file()), "all written");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_texture_with_an_alpha_tail_exports_with_its_alpha() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let texture = open_first(&archive, &ALPHA_TAILS)?;
    let dir = tempfile::tempdir()?;
    assert!(texture.has_alpha_tail(), "the texture has an alpha tail");
    let header = *texture.header();

    let out = dir.path().join("normal.png");
    let report = texture.png_to_file(&out, 0)?;
    assert!(report.alpha_tail, "the tail was merged");
    let (width, height, pixels) = read_png(&out)?;
    assert_eq!((width, height), (header.width(), header.height()), "size");
    assert!(varies(3, &pixels), "the alpha channel carries the tail");
    assert!(varies(2, &pixels), "the blue channel is reconstructed");
    assert!(pixels.iter().skip(2).step_by(4).any(|&blue| blue > 0));

    let tail = texture.alpha_tail().ok_or("an alpha tail")??;
    assert!(
        tail.name().to_ascii_lowercase().ends_with(".dds.a"),
        "{}",
        tail.name()
    );
    let image = tail.decode_mip(0)?;
    assert_eq!(
        (image.width(), image.height()),
        (width, height),
        "tail size"
    );

    let level = header.mip_count().saturating_sub(1).min(2);
    let small = dir.path().join("small.png");
    texture.png_to_file(&small, level)?;
    let (small_width, small_height, small_pixels) = read_png(&small)?;
    assert_eq!(
        (small_width, small_height),
        (
            (header.width() >> level).max(1),
            (header.height() >> level).max(1)
        ),
        "a smaller mip"
    );
    assert!(varies(3, &small_pixels), "the tail reaches every mip");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_inventory_probe_cubemap_with_a_cryengine_trailer_merges_and_exports() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry: &Entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path().eq_ignore_ascii_case(PROBE))
        .ok_or("the probe cubemap")?;
    let texture = Texture::from_archive(&archive, PROBE)?;
    let dir = tempfile::tempdir()?;
    assert_eq!(texture.part_count()?, 0, "every mip is in the header Entry");
    let expected = merged_size(&texture)?;
    assert_eq!(
        entry.uncompressed_size(),
        expected + CRYENGINE_TRAILER_LEN,
        "the Entry ends with the eight bytes of the trailer"
    );

    let mut data = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut data)?;
    assert_eq!(
        data.last_chunk::<8>(),
        Some(b"CExtCEnd"),
        "the eight bytes are the trailer markers"
    );

    let merged = dir.path().join("probe.dds");
    let report = texture.merge_to_file(&merged)?;
    assert_eq!(report.size, expected, "the merged size");
    assert_eq!(std::fs::metadata(&merged)?.len(), expected, "the file");

    let cross = dir.path().join("probe.png");
    texture.png_to_file_as(&cross, 0, CubemapLayout::Cross)?;
    let (width, height, pixels) = read_png(&cross)?;
    assert_eq!((width, height), (CUBE_SIDE * 4, CUBE_SIDE * 3), "the cross");
    assert!(varies(0, &pixels), "not one flat colour");
    Ok(())
}
