// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `cryxml`, `catalog`, a Query export and `chf` refuse an output inside a Star Citizen
//! installation and create nothing there. The installations are synthetic folders in a temp folder.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, TestChf, TestElement,
    TestEntry, TestRecord, TestStruct, TestType, TestValue,
};
use predicates::prelude::predicate;

use super::catalog_support::write_archive;

type TestResult = Result<(), Box<dyn Error>>;

const REFUSAL: &str = "Star Citizen installation";

fn installation(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let library = root.join("StarCitizen");
    std::fs::create_dir_all(library.join("LIVE").join("Bin64"))?;
    Ok(library)
}

fn new_output(library: &Path, name: &str) -> PathBuf {
    library.join("LIVE").join("export").join(name)
}

fn assert_nothing_created(out: &Path) -> TestResult {
    assert!(!out.exists(), "no file is created inside the installation");
    let parent = out.parent().ok_or("no parent")?;
    assert!(!parent.exists(), "no folder is created inside the installation");
    Ok(())
}

fn cryxml_input(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let input = dir.join("vehicle.xml");
    let bytes =
        TestElement::new("Vehicle").to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    std::fs::write(&input, bytes)?;
    Ok(input)
}

#[test]
fn cryxml_refuses_an_output_inside_the_installation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let library = installation(dir.path())?;
    let input = cryxml_input(dir.path())?;
    let out = new_output(&library, "vehicle.xml");

    p4kx_testkit::p4kx()?
        .env("P4KX_PROTECT_FOLDERS", &library)
        .arg("cryxml")
        .arg(&input)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(REFUSAL));

    assert_nothing_created(&out)
}

#[test]
fn catalog_refuses_an_output_inside_the_installation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let library = dir.path().to_path_buf();
    let out = new_output(&library, "catalog.json");

    p4kx_testkit::p4kx()?
        .env("P4KX_PROTECT_FOLDERS", &library)
        .arg("catalog")
        .arg(&archive)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(REFUSAL));

    assert_nothing_created(&out)
}

#[test]
fn a_query_export_refuses_an_output_inside_the_installation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let library = installation(dir.path())?;
    let archive = library.join("LIVE").join("Data.p4k");
    let database = DataForgeBuilder::new()
        .structure(TestStruct::new("Item").field("name", TestType::String))
        .record(
            TestRecord::new(1, "Item", "Item.Box", "libs/items/box.xml")
                .value("name", TestValue::text("Box")),
        );
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database.build()))
        .write_to(&archive)?;
    let out = new_output(&library, "items.csv");

    p4kx_testkit::p4kx()?
        .args(["dcb", "query"])
        .arg(&archive)
        .arg("at \"*\" select name")
        .args(["--format", "csv", "--out"])
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(REFUSAL));

    assert_nothing_created(&out)
}

#[test]
fn chf_from_json_refuses_an_output_inside_the_installation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let library = installation(dir.path())?;
    let chf = dir.path().join("character.chf");
    std::fs::write(&chf, TestChf::v8().to_bytes())?;
    let json = dir.path().join("character.json");
    p4kx_testkit::p4kx()?
        .args(["chf", "to-json"])
        .arg(&chf)
        .arg("--out")
        .arg(&json)
        .assert()
        .success();
    let out = new_output(&library, "character.chf");

    p4kx_testkit::p4kx()?
        .env("P4KX_PROTECT_FOLDERS", &library)
        .args(["chf", "from-json"])
        .arg(&json)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(REFUSAL));

    assert_nothing_created(&out)
}

#[test]
fn a_folder_added_to_the_settings_is_protected_and_a_plain_one_is_not() -> TestResult {
    let dir = tempfile::tempdir()?;
    let library = installation(dir.path())?;
    let workspace = dir.path().join("Workspace");
    std::fs::create_dir_all(&workspace)?;
    let config = dir.path().join("config");
    std::fs::create_dir_all(&config)?;
    let quoted = |path: &Path| path.display().to_string().replace('\\', "/");
    std::fs::write(
        config.join("settings.toml"),
        format!(
            "[channels]\nfolders = [\"{}\", \"{}\"]\n",
            quoted(&library),
            quoted(&workspace)
        ),
    )?;
    let input = cryxml_input(dir.path())?;
    let refused = new_output(&library, "vehicle.xml");
    let allowed = workspace.join("vehicle.xml");

    p4kx_testkit::p4kx()?
        .env("P4KX_CONFIG_DIR", &config)
        .arg("cryxml")
        .arg(&input)
        .arg("--out")
        .arg(&refused)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(REFUSAL));
    p4kx_testkit::p4kx()?
        .env("P4KX_CONFIG_DIR", &config)
        .arg("cryxml")
        .arg(&input)
        .arg("--out")
        .arg(&allowed)
        .assert()
        .code(0);

    assert_nothing_created(&refused)?;
    assert!(allowed.is_file(), "a plain manual folder stays writable");
    Ok(())
}
