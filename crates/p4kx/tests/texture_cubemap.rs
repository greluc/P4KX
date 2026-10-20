//! `p4kx texture merge|png` end to end for cubemaps and alpha tails: merged byte layout and PNG
//! pixel snapshots.

use std::error::Error;
use std::fmt::Write as _;
use std::fs::File;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestTextureFormat};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

const HEADER: &str = "Data\\Textures\\probe_cm.dds";
const FACES: [&str; 6] = ["posx", "negx", "posy", "negy", "posz", "negz"];

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
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
    let mut text = format!("{} x {}\n", info.width, info.height);
    for row in pixels
        .chunks_exact(info.width as usize * 4)
        .take(info.height as usize)
    {
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

fn cubemap() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyBgra8, 2, 2)
        .cubemap()
        .split(1)
}

#[test]
fn merge_writes_a_split_cubemap_face_by_face() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = cubemap();
    let archive = write_archive(dir.path(), &dds)?;
    let out = dir.path().join("probe_cm.dds");

    p4kx()?
        .args(["texture", "merge"])
        .arg(&archive)
        .arg(HEADER)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("and 1 parts"));

    let merged = std::fs::read(&out)?;
    assert_eq!(
        merged.len(),
        128 + 6 * 20,
        "header and six faces of 16 + 4 bytes"
    );
    for face in 0..6 {
        let start = 128 + face as usize * 20;
        assert_eq!(
            merged.get(start..start + 20),
            Some(
                [dds.surface_mip(face, 0), dds.surface_mip(face, 1)]
                    .concat()
                    .as_slice()
            ),
            "face {face}: mip 0 from the part, then mip 1 from the header"
        );
    }
    Ok(())
}

#[test]
fn png_writes_a_cubemap_as_six_face_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), &cubemap())?;
    let out = dir.path().join("probe.png");

    p4kx()?
        .args(["texture", "png"])
        .arg(&archive)
        .arg(HEADER)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("probe_posx.png"))
        .stdout(predicate::str::contains("probe_negz.png"));

    assert!(!out.exists(), "no single file");
    let mut snapshot = String::new();
    for face in FACES {
        writeln!(
            snapshot,
            "{face}: {}",
            pixel_snapshot(&dir.path().join(format!("probe_{face}.png")))?
        )?;
    }
    insta::assert_snapshot!(snapshot);
    Ok(())
}

#[test]
fn png_cross_writes_a_cubemap_as_one_horizontal_cross() -> TestResult {
    let dir = tempfile::tempdir()?;
    let header = cubemap().write_files(dir.path(), "probe_cm.dds")?;
    let out = dir.path().join("probe.png");

    let output = p4kx()?
        .args(["--json", "texture", "png"])
        .arg(&header)
        .arg("--out")
        .arg(&out)
        .arg("--cross")
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["width"], 8, "four faces wide");
    assert_eq!(json["height"], 6, "three faces high");
    assert_eq!(
        json["files"],
        serde_json::json!([out.display().to_string()]),
        "one file"
    );
    insta::assert_snapshot!(pixel_snapshot(&out)?);
    Ok(())
}

#[test]
fn png_merges_the_alpha_tail_into_the_alpha_channel() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = SplitDds::new(TestTextureFormat::LegacyBgra8, 4, 2)
        .split(1)
        .alpha(TestTextureFormat::R8);
    let archive = write_archive(dir.path(), &dds)?;
    let out = dir.path().join("glass.png");

    p4kx()?
        .args(["texture", "png"])
        .arg(&archive)
        .arg(HEADER)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("with its alpha tail"));

    insta::assert_snapshot!(pixel_snapshot(&out)?);
    Ok(())
}

#[test]
fn an_alpha_tail_in_a_format_without_a_decoder_is_named() -> TestResult {
    let dir = tempfile::tempdir()?;
    let dds = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 8)
        .split(1)
        .alpha(TestTextureFormat::Dx10Pixel {
            dxgi_format: 3,
            pixel_bytes: 16,
        });
    let header = dds.write_files(dir.path(), "glass.dds")?;
    let out = dir.path().join("glass.png");

    p4kx()?
        .args(["texture", "png"])
        .arg(&header)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("glass.dds.a"));

    assert!(!out.exists(), "nothing is written");
    Ok(())
}
