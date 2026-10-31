// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Slot 17 of the DNA rig of the stock male head: the numbers ADR-0293 cites to leave it unapplied.

use std::error::Error;
use std::io::Read;

use p4kx_character::DnaRig;
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};

const HEAD: &str = "objects/characters/human/heads/male/pu/protos_human_male_face_t1_pu/protos_human_male_face_t1_pu_head.dna";
type TestResult = Result<(), Box<dyn Error>>;

const SLOT: usize = 17;
const TABLE_AT: usize = 0x40;
const ROW: usize = 8;
const VERTICES: usize = 5_584;
/// The 61 heads of the rig and one more block.
const BLOCKS: usize = 62;

fn read_dna(archive: &Archive) -> Result<Vec<u8>, Box<dyn Error>> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| {
            entry
                .path()
                .to_ascii_lowercase()
                .replace('\\', "/")
                .ends_with(HEAD)
        })
        .ok_or("the Archive has no stock male head rig")?;
    let mut bytes = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn pointer(bytes: &[u8], slot: usize) -> Option<usize> {
    let at = TABLE_AT + slot * 8;
    let stored = u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?);
    usize::try_from(stored).ok().filter(|&start| start != 0)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn slot_17_is_eight_bytes_per_vertex_and_never_zero_where_a_blendshape_moves() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let bytes = read_dna(&archive)?;
    let start = pointer(&bytes, SLOT).ok_or("no slot 17")?;
    let end = (0..32)
        .filter_map(|slot| pointer(&bytes, slot))
        .filter(|&other| other > start)
        .min()
        .unwrap_or(bytes.len());
    let data = bytes
        .get(start..end)
        .ok_or("slot 17 lies outside the file")?;
    assert_eq!(data.len(), BLOCKS * VERTICES * ROW);
    let count = |wanted: u8| {
        data.iter()
            .map(|&byte| usize::from(byte == wanted))
            .sum::<usize>()
    };
    let (zeros, full) = (count(0), count(255));
    println!("slot 17: {} bytes, {zeros} zero, {full} at 255", data.len());

    let morphs = DnaRig::read(&bytes)?.expression_morphs()?;
    let (mut moves, mut empty_rows, mut full_rows, mut sum) = (0_usize, 0_usize, 0_usize, 0_usize);
    for blendshape in (1..morphs.blendshapes()).step_by(7) {
        let Some(cell) = morphs.cell(0, blendshape) else {
            continue;
        };
        for (vertex, _) in cell.moves() {
            let at = usize::try_from(vertex)? * ROW;
            let row = data.get(at..at + ROW).ok_or("a vertex beyond slot 17")?;
            moves += 1;
            empty_rows += usize::from(row.iter().all(|&byte| byte == 0));
            full_rows += usize::from(row.contains(&255));
            sum += row.iter().map(|&byte| usize::from(byte)).sum::<usize>();
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "a mean for the report")]
    let mean = sum as f64 / (moves * ROW) as f64;
    println!(
        "{moves} moves: {empty_rows} on empty rows, {full_rows} on rows with a 255, mean byte {mean:.1}"
    );
    assert_eq!(moves, 24_274);
    assert_eq!((empty_rows, full_rows), (0, 0));
    assert!((20.0..45.0).contains(&mean), "{mean}");
    Ok(())
}
