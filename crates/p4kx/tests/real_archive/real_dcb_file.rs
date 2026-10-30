// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The DataForge of a real Archive, written to a file of its own, reads the same from the file as
//! from the Archive.

use std::error::Error;
use std::fs::File;
use std::io;

use p4kx_extraction::{Archive, DataForgeEntry, is_dataforge_file};
use p4kx_testkit::real_archive_path;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const QUERY: &str = "from EntityClassDefinition at \"libs/foundry/records/entities/spaceships/*\" \
select Components[VehicleComponentParams].crewSize";

fn run(args: &[&str], input: &std::path::Path) -> Result<Value, Box<dyn Error>> {
    let output = p4kx_testkit::p4kx()?
        .arg("--json")
        .arg("dcb")
        .args(args)
        .arg(input)
        .args(if args == ["query"] {
            vec![QUERY]
        } else {
            vec![]
        })
        .output()?;
    assert_eq!(output.status.code(), Some(0), "exit code: {output:?}");
    let mut value: Value = serde_json::from_slice(&output.stdout)?;
    if let Some(object) = value.as_object_mut() {
        object.remove("archive");
        object.remove("entry");
    }
    Ok(value)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_dataforge_file_of_a_real_archive_reads_like_the_archive() -> TestResult {
    let path = real_archive_path()?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::real_archive_key()?))?;
    let entry = DataForgeEntry::locate(&archive).ok_or("the Archive holds no DataForge")?;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("Game2.dcb");
    io::copy(&mut archive.open_entry(entry)?, &mut File::create(&file)?)?;

    assert!(is_dataforge_file(&file)?, "the real header is recognised");
    for command in [&["info"][..], &["query"][..]] {
        assert_eq!(
            run(command, &file)?,
            run(command, &path)?,
            "{command:?} from the file and from the Archive"
        );
    }
    Ok(())
}
