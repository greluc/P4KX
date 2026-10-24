// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The last eight bytes of the CHF files of a real Archive survive a change of the body.

use std::error::Error;
use std::io::Read;

use p4kx_assembly::character::{CHF_SIZE, Chf};
use p4kx_extraction::Archive;
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn files_with_non_zero_tail_bytes_keep_them_when_the_body_changes() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let (mut files, mut tailed, mut kept, mut refused) = (0_usize, 0_usize, 0_usize, 0_usize);
    let mut examples = Vec::new();
    for entry in archive.entries() {
        if !entry.path().to_ascii_lowercase().ends_with(".chf") {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(entry)?.read_to_end(&mut bytes)?;
        files += 1;
        let tail = bytes.get(CHF_SIZE - 8..).ok_or("a short file")?;
        if tail.iter().all(|byte| *byte == 0) {
            continue;
        }
        tailed += 1;
        if examples.len() < 5 {
            examples.push(format!("{}: {tail:02x?}", entry.path()));
        }
        let mut chf = Chf::read(&bytes)?;
        let Some(dna) = chf.character.dna.as_mut() else {
            continue;
        };
        dna.max_head_id = dna.max_head_id.wrapping_add(1);
        match chf.write() {
            Ok(written) => {
                assert_eq!(written.get(CHF_SIZE - 8..), Some(tail), "{}", entry.path());
                assert_eq!(Chf::read(&written)?.character, chf.character);
                kept += 1;
            }
            Err(_) => refused += 1,
        }
    }
    println!(
        "{files} CHF files, {tailed} with non-zero bytes in the last 8, {kept} kept them after a \
         change, {refused} refused; examples: {examples:#?}"
    );
    assert!(files > 0, "the Archive holds CHF files");
    assert_eq!(refused, 0, "no changed body is refused");
    assert!(kept > 0, "some file keeps its tail after a change");
    Ok(())
}
