// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `RigLogic` chunks of the head skins the game ships: the stored hash of every control is the
//! CRC-32 of its name.

use std::error::Error;
use std::io::Read;

use p4kx_character::RigLogic;
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the count is the measurement")]
fn the_stored_control_hashes_of_every_head_skin_are_the_crcs_of_the_names() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let mut checked = 0_usize;
    for entry in archive.entries() {
        let path = entry.path().to_ascii_lowercase().replace('\\', "/");
        let is_skin = std::path::Path::new(&path)
            .extension()
            .is_some_and(|extension| extension == "skin");
        if !is_skin || !path.contains("/heads/") {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(entry)?.read_to_end(&mut bytes)?;
        let Ok(chunks) = ChunkFile::parse(bytes.as_slice()) else {
            continue;
        };
        let Some((index, _)) = chunks.chunks_of(ivo::RIG_LOGIC).next() else {
            continue;
        };
        let data = chunks
            .chunk_data(index)
            .ok_or("the chunk lies in the file")?;
        let logic = RigLogic::read(data).map_err(|error| format!("{path}: {error}"))?;
        assert_ne!(logic.controls(), 0, "{path}");
        checked += 1;
    }
    println!("{checked} head skins with a RigLogic chunk");
    assert_ne!(checked, 0);
    Ok(())
}
