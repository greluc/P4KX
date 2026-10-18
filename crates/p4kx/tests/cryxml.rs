//! `p4kx cryxml` end to end.

use std::error::Error;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

fn vehicle() -> TestElement {
    TestElement::new("Vehicle")
        .attribute("name", "AEGS_Gladius")
        .attribute("displayName", "@vehicle_NameAEGS_Gladius")
        .child(
            TestElement::new("Parts")
                .child(
                    TestElement::new("Part")
                        .attribute("name", "Hull")
                        .attribute("damageMax", "2500"),
                )
                .child(TestElement::new("Part").attribute("name", "Wing_Left & Right")),
        )
        .child(TestElement::new("Description").content("Light <fighter>"))
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join(name);
    std::fs::write(&path, bytes)?;
    Ok(path)
}

fn stdout_of(file: &Path) -> Result<String, Box<dyn Error>> {
    let output = Command::cargo_bin("p4kx")?
        .arg("cryxml")
        .arg(file)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "exit code: {output:?}");
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn cryxml_writes_xml_to_stdout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = vehicle().to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let file = write_file(dir.path(), "vehicle.xml", &bytes)?;

    let xml = stdout_of(&file)?;

    insta::assert_snapshot!(xml);
    Ok(())
}

#[test]
fn every_signature_and_byte_order_gives_the_same_xml() -> TestResult {
    let dir = tempfile::tempdir()?;
    let reference = vehicle().to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let expected = stdout_of(&write_file(dir.path(), "reference.xml", &reference)?)?;
    for signature in [
        CryXmlSignature::CryXmlB,
        CryXmlSignature::CryXml,
        CryXmlSignature::Cry3Sdk,
    ] {
        for byte_order in [CryXmlByteOrder::Little, CryXmlByteOrder::Big] {
            let bytes = vehicle().to_cryxml(signature, byte_order);
            let file = write_file(dir.path(), "variant.xml", &bytes)?;

            let xml = stdout_of(&file)?;

            assert_eq!(xml, expected, "{signature:?} {byte_order:?}");
        }
    }
    Ok(())
}

#[test]
fn cryxml_writes_to_an_output_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = vehicle().to_cryxml(CryXmlSignature::Cry3Sdk, CryXmlByteOrder::Big);
    let file = write_file(dir.path(), "vehicle.xml", &bytes)?;
    let out = dir.path().join("converted.xml");

    Command::cargo_bin("p4kx")?
        .arg("cryxml")
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("CRY3SDK"));

    assert_eq!(
        std::fs::read_to_string(&out)?,
        stdout_of(&file)?,
        "same XML"
    );
    Ok(())
}

#[test]
fn text_xml_is_written_byte_identical() -> TestResult {
    let dir = tempfile::tempdir()?;
    let text = b"\xEF\xBB\xBF<?xml version=\"1.0\"?>\r\n<a b='1'>\r\n</a>";
    let file = write_file(dir.path(), "text.xml", text)?;

    let output = Command::cargo_bin("p4kx")?
        .arg("cryxml")
        .arg(&file)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "exit code: {output:?}");
    assert_eq!(output.stdout, text, "byte-identical");
    Ok(())
}

#[test]
fn truncated_cryxml_exits_with_code_3_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = vehicle().to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let file = write_file(
        dir.path(),
        "cut.xml",
        bytes.get(..bytes.len() / 2).ok_or("short")?,
    )?;
    let out = dir.path().join("converted.xml");

    Command::cargo_bin("p4kx")?
        .arg("cryxml")
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("truncated CryXml"));

    assert!(!out.exists(), "no output file");
    Ok(())
}

#[test]
fn a_file_that_is_not_xml_exits_with_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_file(dir.path(), "notes.txt", b"plain text")?;

    Command::cargo_bin("p4kx")?
        .arg("cryxml")
        .arg(&file)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("not CryXml"));
    Ok(())
}

#[test]
fn a_missing_file_exits_with_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;

    Command::cargo_bin("p4kx")?
        .arg("cryxml")
        .arg(dir.path().join("missing.xml"))
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot read"));
    Ok(())
}

#[test]
fn json_without_an_output_path_exits_with_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = vehicle().to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let file = write_file(dir.path(), "vehicle.xml", &bytes)?;

    Command::cargo_bin("p4kx")?
        .arg("--json")
        .arg("cryxml")
        .arg(&file)
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("--out"));
    Ok(())
}
