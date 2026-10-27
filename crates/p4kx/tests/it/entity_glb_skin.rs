// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity glb --json` reports a part on a skin port with the placement code `skin`.

use std::error::Error;

use p4kx_testkit::{ArchiveBuilder, SKIN_BODY, skin_attachment_entries};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_skin_port_is_listed_with_the_placement_code_skin() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    skin_attachment_entries()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&archive)?;
    let out = dir.path().join("body.glb");

    let output = p4kx_testkit::p4kx()?
        .arg("--json")
        .args(["entity", "glb"])
        .arg(&archive)
        .arg(SKIN_BODY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let codes = |path: &str| {
        report["placements"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|placement| placement["path"] == path)
            .map(|placement| placement["placement"].clone())
    };
    for path in ["legs", "gone", "bare"] {
        assert_eq!(codes(path), Some(Value::from("skin")), "{path}");
    }
    assert_eq!(codes("hand"), Some(Value::from("helper")));
    Ok(())
}
