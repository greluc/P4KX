//! `p4kx extract --convert` over a synthetic Archive with split DDS textures.

use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat};
use predicates::prelude::{PredicateBooleanExt, predicate};

type TestResult = Result<(), Box<dyn Error>>;

const PANEL: &str = "Data\\Textures\\panel_diff.dds";
const BROKEN: &str = "Data\\Textures\\broken_ddna.dds";

fn panel() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 8).split(2)
}

struct Fixture {
    _dir: tempfile::TempDir,
    archive: PathBuf,
    out: PathBuf,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    let mut broken = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 8)
        .split(1)
        .entries(BROKEN);
    broken.truncate(1);
    panel()
        .entries(PANEL)
        .into_iter()
        .chain(broken)
        .chain([TestEntry::stored("Data\\readme.txt", b"hello".to_vec())])
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&archive)?;
    let out = dir.path().join("out");
    Ok(Fixture {
        _dir: dir,
        archive,
        out,
    })
}

fn extract(fixture: &Fixture) -> Result<Command, Box<dyn Error>> {
    let mut command = Command::cargo_bin("p4kx")?;
    command
        .arg("extract")
        .arg(&fixture.archive)
        .args(["--filter", "Data\\"])
        .arg("--out")
        .arg(&fixture.out);
    Ok(command)
}

fn files(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder)? {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let relative = path
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                found.push(relative);
            }
        }
    }
    found.sort();
    Ok(found)
}

fn png_size(path: &Path) -> Result<(u32, u32), Box<dyn Error>> {
    let reader = png::Decoder::new(std::io::BufReader::new(File::open(path)?)).read_info()?;
    let info = reader.info();
    Ok((info.width, info.height))
}

fn merged(dds: &SplitDds) -> Vec<u8> {
    let mut merged = dds
        .header_bytes()
        .get(..dds.header_len())
        .unwrap_or_default()
        .to_vec();
    for level in 0..5 {
        merged.extend_from_slice(&dds.mip(level));
    }
    merged
}

#[test]
fn convert_merges_textures_and_exports_png_once_per_texture() -> TestResult {
    let fixture = fixture()?;

    extract(&fixture)?
        .arg("--convert")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "extracted 5 (3 converted, 1 not converted), skipped 0, failed 0, cancelled 0",
        ))
        .stderr(predicate::str::contains(format!(
            "not converted {BROKEN}: cannot convert the texture: {BROKEN}.1 is missing; \
             its raw Entries were written"
        )));

    assert_eq!(
        files(&fixture.out)?,
        [
            "Data/Textures/broken_ddna.dds",
            "Data/Textures/panel_diff.dds",
            "Data/Textures/panel_diff.png",
            "Data/readme.txt",
        ],
        "one DDS and one PNG for the texture, the raw header of the broken one"
    );
    assert_eq!(
        fs::read(fixture.out.join("Data/Textures/panel_diff.dds"))?,
        merged(&panel()),
        "merged DDS"
    );
    assert_eq!(
        png_size(&fixture.out.join("Data/Textures/panel_diff.png"))?,
        (16, 8),
        "the largest mip"
    );
    Ok(())
}

#[test]
fn textures_and_mip_choose_the_outputs() -> TestResult {
    let fixture = fixture()?;

    extract(&fixture)?
        .args(["--convert", "--textures", "png,raw", "--mip", "1"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "extracted 5 (4 converted), skipped 0, failed 0, cancelled 0",
        ));

    assert_eq!(
        files(&fixture.out)?,
        [
            "Data/Textures/broken_ddna.dds",
            "Data/Textures/broken_ddna.png",
            "Data/Textures/panel_diff.dds",
            "Data/Textures/panel_diff.dds.1",
            "Data/Textures/panel_diff.dds.2",
            "Data/Textures/panel_diff.png",
            "Data/readme.txt",
        ],
        "the raw Entries and the PNGs; mip 1 of the broken texture needs no part"
    );
    assert_eq!(
        fs::read(fixture.out.join("Data/Textures/panel_diff.dds"))?,
        panel().header_bytes(),
        "the raw header"
    );
    assert_eq!(
        png_size(&fixture.out.join("Data/Textures/panel_diff.png"))?,
        (8, 4),
        "mip 1"
    );
    Ok(())
}

#[test]
fn textures_need_convert() -> TestResult {
    let fixture = fixture()?;

    extract(&fixture)?
        .args(["--textures", "merge"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--convert"));
    Ok(())
}

#[test]
fn the_json_report_lists_textures_that_were_not_converted() -> TestResult {
    let fixture = fixture()?;

    let output = extract(&fixture)?
        .args(["--convert", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value = serde_json::from_slice(&output)?;
    assert_eq!(
        report["converted"], 3,
        "the Entries of the converted texture"
    );
    assert_eq!(
        report["unconverted"][0]["path"], BROKEN,
        "the broken texture"
    );
    assert_eq!(
        report["unconverted"][0]["error"],
        format!("cannot convert the texture: {BROKEN}.1 is missing"),
        "and why"
    );
    Ok(())
}

#[test]
fn without_convert_textures_are_written_raw() -> TestResult {
    let fixture = fixture()?;

    extract(&fixture)?
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted").not());

    assert_eq!(files(&fixture.out)?.len(), 5, "the raw Entries");
    Ok(())
}
