// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The last bytes of a CHF file survive a change of the body.

use std::error::Error;

use p4kx_character::{CHF_SIZE, Chf, ChfError, ColorParam};
use p4kx_testkit::TestChf;

type TestResult = Result<(), Box<dyn Error>>;

const TAIL: [u8; 8] = [0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef];

fn file_with_tail(tail: [u8; 8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let base = TestChf::v7();
    let frame_len = usize::try_from(u32::from_le_bytes(
        base.to_bytes().get(8..12).ok_or("no header")?.try_into()?,
    ))?;
    let gap = CHF_SIZE - 16 - frame_len - tail.len();
    let mut padding = vec![0; gap];
    padding.extend_from_slice(&tail);
    Ok(TestChf { padding, ..base }.to_bytes())
}

fn crc_is_correct(file: &[u8]) -> Result<bool, Box<dyn Error>> {
    let stored = u32::from_le_bytes(file.get(4..8).ok_or("no CRC")?.try_into()?);
    Ok(stored == crc32c::crc32c(file.get(16..).ok_or("no data")?))
}

fn edited(file: &[u8]) -> Result<Chf, Box<dyn Error>> {
    let mut chf = Chf::read(file)?;
    chf.character.dna.as_mut().ok_or("no DNA")?.max_head_id = 77;
    Ok(chf)
}

#[test]
fn a_file_with_a_tail_is_written_back_unchanged() -> TestResult {
    let file = file_with_tail(TAIL)?;

    let chf = Chf::read(&file)?;

    assert_eq!(chf.write()?, file, "directly");
    assert_eq!(
        Chf::from_json(chf.to_json()?.as_bytes())?.write()?,
        file,
        "through JSON"
    );
    Ok(())
}

#[test]
fn a_changed_body_keeps_the_last_eight_bytes_with_a_correct_crc() -> TestResult {
    let file = file_with_tail(TAIL)?;
    let chf = edited(&file)?;

    let written = chf.write()?;

    assert_eq!(written.len(), CHF_SIZE, "size");
    assert_eq!(written.get(CHF_SIZE - 8..), Some(&TAIL[..]), "the tail");
    assert_ne!(written.get(16..32), file.get(16..32), "the frame is new");
    assert!(crc_is_correct(&written)?, "CRC-32C");
    assert_eq!(
        Chf::read(&written)?.character,
        chf.character,
        "the change is read back"
    );
    let reread = Chf::read(&written)?;
    assert_eq!(
        Chf::from_json(edited(&file)?.to_json()?.as_bytes())?.write()?,
        written,
        "through JSON"
    );
    assert_eq!(reread.write()?, written, "and stable");
    Ok(())
}

#[test]
fn a_changed_body_of_a_file_with_a_zero_tail_has_a_zero_tail() -> TestResult {
    let file = TestChf::v7().to_bytes();

    let written = edited(&file)?.write()?;

    assert_eq!(
        written.get(CHF_SIZE - 8..),
        Some(&[0_u8; 8][..]),
        "zeros, as before"
    );
    assert!(crc_is_correct(&written)?, "CRC-32C");
    Ok(())
}

fn grown(chf: &Chf, colors: u32) -> Result<Chf, Box<dyn Error>> {
    let mut grown = chf.clone();
    let sub = grown
        .character
        .materials
        .first_mut()
        .and_then(|material| material.sub_materials.first_mut())
        .ok_or("no sub-material")?;
    let mut state = 0x2545_f491_u32;
    for _ in 0..colors {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        sub.colors.push(ColorParam {
            name: state,
            label: p4kx_character::Label,
            rgba: [0; 4],
        });
    }
    Ok(grown)
}

fn frame_len(file: &[u8]) -> Result<usize, Box<dyn Error>> {
    Ok(usize::try_from(u32::from_le_bytes(
        file.get(8..12).ok_or("no header")?.try_into()?,
    ))?)
}

fn colors_reaching_the_tail(plain: &Chf) -> Result<u32, Box<dyn Error>> {
    let window = CHF_SIZE - 16 - 8 + 1..=CHF_SIZE - 16;
    for colors in 0..2000 {
        let length = frame_len(&grown(plain, colors)?.write()?)?;
        if window.contains(&length) {
            return Ok(colors);
        }
        if length > *window.end() {
            break;
        }
    }
    Err("no body compresses into the last 8 bytes of the file".into())
}

#[test]
fn a_changed_body_whose_frame_reaches_the_tail_is_an_error() -> TestResult {
    let tailed = Chf::read(&file_with_tail(TAIL)?)?;
    let plain = Chf::read(&TestChf::v7().to_bytes())?;
    let colors = colors_reaching_the_tail(&plain)?;

    let overlapping = grown(&tailed, colors)?.write();

    assert!(
        matches!(&overlapping, Err(ChfError::Unwritable { reason }) if reason.contains("last 8 bytes")),
        "{overlapping:?}"
    );
    let free = grown(&plain, colors)?.write()?;
    assert!(
        crc_is_correct(&free)?,
        "the same body fits a file with a zero tail"
    );
    Ok(())
}

#[test]
fn a_tail_inside_the_original_frame_is_not_kept() -> TestResult {
    let plain = Chf::read(&TestChf::v7().to_bytes())?;
    let colors = colors_reaching_the_tail(&plain)?;
    let reaching = grown(&plain, colors)?.write()?;
    assert_eq!(
        reaching.get(CHF_SIZE - 8..).map(<[u8]>::len),
        Some(8),
        "the frame fills the end of the file"
    );

    let rewritten = edited(&reaching)?.write()?;

    assert!(crc_is_correct(&rewritten)?, "CRC-32C");
    assert_eq!(
        Chf::read(&rewritten)?.character,
        edited(&reaching)?.character,
        "the new frame is intact: the old frame bytes were not written back as a tail"
    );
    Ok(())
}

#[test]
fn a_new_file_without_container_has_a_zero_tail() -> TestResult {
    let character = Chf::read(&file_with_tail(TAIL)?)?.character;

    let written = Chf::new(character).write()?;

    assert_eq!(written.get(CHF_SIZE - 8..), Some(&[0_u8; 8][..]), "zeros");
    Ok(())
}
