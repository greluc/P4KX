//! `p4kx ui render --svg` end to end: the document structure for the synthetic panel, the same
//! output for the same input, the size and background options, the escaped Entry name, and the
//! report.

use crate::ui_panel;

use std::error::Error;
use std::fs;

use p4kx_testkit::{ArchiveBuilder, TestEntry};
use predicates::prelude::predicate;
use roxmltree::Document;
use serde_json::Value;
use ui_panel::{p4kx, panel, write_panel};

type TestResult = Result<(), Box<dyn Error>>;

fn count(document: &Document<'_>, name: &str) -> usize {
    document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == name)
        .count()
}

#[test]
fn a_panel_becomes_a_well_formed_document_with_every_kind_of_element() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    let out = dir.path().join("panel.svg");
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .arg("--svg")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("64 x 48 px; 1 elements left out"))
        .stderr(predicate::str::contains(
            "skipped: definition 50 at depth 7: DefineButton2 definitions are not drawn",
        ));
    let text = fs::read_to_string(&out)?;
    let document = Document::parse(&text)?;
    let root = document.root_element();
    assert_eq!(root.attribute("width"), Some("64"));
    assert_eq!(root.attribute("height"), Some("48"));
    assert_eq!(root.attribute("viewBox"), Some("0 0 64 48"));
    assert_eq!(
        count(&document, "linearGradient"),
        2,
        "the backdrop, also inverted"
    );
    assert_eq!(count(&document, "radialGradient"), 1, "the dial");
    assert_eq!(count(&document, "image"), 1, "the badge bitmap once");
    assert!(count(&document, "pattern") >= 1, "the badge fill");
    assert_eq!(count(&document, "mask"), 1, "the mask of the stripe");
    assert!(
        document
            .descendants()
            .any(|node| node.attribute("stroke").is_some()),
        "the stroked dial"
    );
    assert!(
        document
            .descendants()
            .any(|node| node.tag_name().name() == "use"
                && node
                    .attribute("transform")
                    .is_some_and(|transform| transform.starts_with("matrix(0.86"))),
        "the rotated sprite contents"
    );
    assert!(!text.contains("<text"), "texts are listed, not drawn");
    Ok(())
}

#[test]
fn the_same_input_gives_the_same_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    let mut outputs = Vec::new();
    for name in ["a.svg", "b.svg"] {
        let out = dir.path().join(name);
        p4kx()?
            .args(["ui", "render"])
            .arg(&file)
            .args(["--svg", "--size", "200x100", "--background", "#10203040"])
            .arg("--out")
            .arg(&out)
            .assert()
            .code(1);
        outputs.push(fs::read(&out)?);
    }
    assert_eq!(outputs[0], outputs[1]);
    Ok(())
}

#[test]
fn size_and_background_set_the_document() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    let out = dir.path().join("wide.svg");
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .args(["--svg", "--size", "128", "--background", "movie"])
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1);
    let text = fs::read_to_string(&out)?;
    let document = Document::parse(&text)?;
    assert_eq!(document.root_element().attribute("width"), Some("128"));
    assert_eq!(document.root_element().attribute("height"), Some("96"));
    let background = document
        .descendants()
        .find(|node| node.tag_name().name() == "rect")
        .ok_or("a background rectangle")?;
    assert_eq!(background.attribute("fill"), Some("#050505"));
    Ok(())
}

#[test]
fn the_entry_name_is_escaped_and_the_json_report_names_the_format() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    let name = "Data\\UI\\<script>&\"x\".gfx";
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(name, panel()))
        .write_to(&archive)?;
    let out = dir.path().join("panel.svg");
    let output = p4kx()?
        .args(["--json", "ui", "render"])
        .arg(&archive)
        .arg(name)
        .arg("--svg")
        .arg("--out")
        .arg(&out)
        .output()?;
    assert_eq!(output.status.code(), Some(1), "a text: {output:?}");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["format"], "svg");
    assert_eq!(
        (report["width"].as_u64(), report["height"].as_u64()),
        (Some(64), Some(48))
    );
    let text = fs::read_to_string(&out)?;
    assert!(!text.contains("<script"), "no markup from the name");
    let document = Document::parse(&text)?;
    assert_eq!(count(&document, "script"), 0);
    let title = document
        .descendants()
        .find(|node| node.tag_name().name() == "title")
        .and_then(|node| node.text())
        .ok_or("a title")?;
    assert_eq!(title, name);
    Ok(())
}

#[test]
fn the_title_of_a_file_is_its_name_without_the_directory() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    let out = dir.path().join("panel.svg");
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .arg("--svg")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1);
    let text = fs::read_to_string(&out)?;
    let document = Document::parse(&text)?;
    let title = document
        .descendants()
        .find(|node| node.tag_name().name() == "title")
        .and_then(|node| node.text())
        .ok_or("a title")?;
    let name = file
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("a file name")?;
    assert_eq!(title, name);
    let directory = dir.path().to_string_lossy();
    assert!(!text.contains(directory.as_ref()), "no local path");
    Ok(())
}

#[test]
fn the_svg_cannot_replace_the_movie_and_bad_sizes_stay_invalid() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_panel(dir.path())?;
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .args(["--svg", "--out"])
        .arg(&file)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is one of the inputs"));
    p4kx()?
        .args(["ui", "render"])
        .arg(&file)
        .args(["--svg", "--size", "20000", "--out"])
        .arg(dir.path().join("x.svg"))
        .assert()
        .code(2);
    assert_eq!(fs::read(&file)?, panel(), "the movie is untouched");
    Ok(())
}
