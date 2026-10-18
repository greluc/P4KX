//! `p4kx texture png` end to end: PNG pixel snapshots per format, mip choice and failures.

use std::error::Error;
use std::fmt::Write as _;
use std::fs::File;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestTextureFormat};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

const HEADER: &str = "Data\\Textures\\panel_diff.dds";

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(Command::cargo_bin("p4kx")?)
}

fn write_archive(folder: &Path, dds: &SplitDds) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    dds.entries(HEADER)
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

/// Reads an 8-bit RGBA PNG and renders its pixels as one line of `RRGGBBAA` values per row.
fn pixel_snapshot(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut reader = png::Decoder::new(std::io::BufReader::new(File::open(path)?)).read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut pixels)?;
    assert_eq!(info.color_type, png::ColorType::Rgba, "RGBA");
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "8 bits per channel");
    let mut text = format!("{} x {}\n", info.width, info.height);
    let row_len = info.width as usize * 4;
    for row in pixels.chunks_exact(row_len).take(info.height as usize) {
        let line: Vec<String> = row
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| {
                pixel
                    .iter()
                    .fold(String::new(), |hex, byte| hex + &format!("{byte:02x}"))
            })
            .collect();
        writeln!(text, "{}", line.join(" "))?;
    }
    Ok(text)
}

fn export_entry(dds: &SplitDds, mip: Option<&str>) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), dds)?;
    let out = dir.path().join("panel_diff.png");
    let mut command = p4kx()?;
    command
        .args(["texture", "png"])
        .arg(&archive)
        .arg(HEADER)
        .arg("--out")
        .arg(&out);
    if let Some(level) = mip {
        command.args(["--mip", level]);
    }
    command.assert().code(0);
    pixel_snapshot(&out)
}

#[test]
fn every_supported_format_exports_its_largest_mip() -> TestResult {
    let formats = [
        ("bc1", TestTextureFormat::LegacyDxt1),
        (
            "bc2",
            TestTextureFormat::Dx10Block {
                dxgi_format: 74,
                block_bytes: 16,
            },
        ),
        ("bc3", TestTextureFormat::LegacyDxt5),
        ("b8g8r8a8", TestTextureFormat::LegacyBgra8),
        ("b8g8r8", TestTextureFormat::LegacyBgr8),
        ("l8a8", TestTextureFormat::LegacyL8A8),
        (
            "b8g8r8x8",
            TestTextureFormat::Dx10Pixel {
                dxgi_format: 88,
                pixel_bytes: 4,
            },
        ),
        (
            "r8",
            TestTextureFormat::Dx10Pixel {
                dxgi_format: 61,
                pixel_bytes: 1,
            },
        ),
        (
            "r16",
            TestTextureFormat::Dx10Pixel {
                dxgi_format: 56,
                pixel_bytes: 2,
            },
        ),
    ];
    for (name, format) in formats {
        let snapshot = export_entry(&SplitDds::new(format, 8, 8).split(2), None)?;
        insta::assert_snapshot!(format!("largest_mip_{name}"), snapshot);
    }
    Ok(())
}

#[test]
fn mip_picks_a_smaller_level_from_a_file_on_disk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = SplitDds::new(TestTextureFormat::LegacyDxt5, 16, 8)
        .split(2)
        .write_files(dir.path(), "panel_diff.dds")?;
    let out = dir.path().join("small.png");

    p4kx()?
        .args(["texture", "png"])
        .arg(&header)
        .arg("--out")
        .arg(&out)
        .args(["--mip", "2"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("wrote mip 2"))
        .stdout(predicate::str::contains("4 x 2"));

    insta::assert_snapshot!(pixel_snapshot(&out)?);
    Ok(())
}

#[test]
fn the_mip_from_a_part_matches_the_mip_from_a_merged_file() -> TestResult {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 16).split(3);
    let unsplit = SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 16);

    for level in ["0", "1", "3"] {
        assert_eq!(
            export_entry(&dds, Some(level))?,
            export_entry(&unsplit, Some(level))?,
            "mip {level}"
        );
    }
    Ok(())
}

#[test]
fn an_unsupported_format_is_named_and_fails_with_code_1() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(
        dir.path(),
        &SplitDds::new(TestTextureFormat::BC7_SRGB, 16, 16).split(1),
    )?;
    let out = dir.path().join("panel_diff.png");

    p4kx()?
        .args(["texture", "png"])
        .arg(&archive)
        .arg(HEADER)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "decoding the texture format BC7_UNORM_SRGB is not supported",
        ));

    assert!(!out.exists(), "nothing is written");
    Ok(())
}

#[test]
fn a_mip_the_texture_does_not_have_fails_with_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = SplitDds::new(TestTextureFormat::LegacyDxt1, 8, 8)
        .write_files(dir.path(), "panel_diff.dds")?;

    p4kx()?
        .args(["texture", "png"])
        .arg(&header)
        .arg("--out")
        .arg(dir.path().join("panel_diff.png"))
        .args(["--mip", "4"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("mip 4 does not exist"));
    Ok(())
}
