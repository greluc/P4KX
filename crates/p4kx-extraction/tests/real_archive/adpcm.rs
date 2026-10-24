// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Every loose real WEM is classified by its format tag, and the IMA ADPCM and PTADPCM ones decode.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use p4kx_extraction::{Archive, Codec, Entry, Wem};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::TestResult;

const HEADER_LEN: usize = 22;
const DECODED_PER_CODEC: usize = 500;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_real_adpcm_wem_decodes_to_wav() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let wems: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().as_bytes();
            path.len() > 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".wem")
        })
        .collect();

    let codecs: Vec<(&Entry, Option<Codec>)> = wems
        .par_iter()
        .map(|entry| (*entry, codec_of(&archive, entry)))
        .collect();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (_, codec) in &codecs {
        let name = codec.map_or_else(
            || "no fmt chunk first".to_owned(),
            |codec| codec.to_string(),
        );
        *counts.entry(name).or_default() += 1;
    }
    let mut adpcm: Vec<&Entry> = Vec::new();
    for codec in [Codec::ImaAdpcm, Codec::PtAdpcm] {
        adpcm.extend(
            codecs
                .iter()
                .filter(|(_, found)| *found == Some(codec))
                .map(|(entry, _)| *entry)
                .take(DECODED_PER_CODEC),
        );
    }
    let failures: Vec<String> = adpcm
        .par_iter()
        .filter_map(|entry| {
            decode(&archive, entry)
                .err()
                .map(|error| format!("{}: {error}", entry.path()))
        })
        .collect();
    writeln!(
        std::io::stderr(),
        "{} loose WEMs by codec: {counts:?}; {} ADPCM WEMs decoded, {} failed",
        wems.len(),
        adpcm.len(),
        failures.len()
    )?;
    assert!(
        failures.is_empty(),
        "{:#?}",
        failures.iter().take(20).collect::<Vec<_>>()
    );
    Ok(())
}

fn codec_of(archive: &Archive, entry: &Entry) -> Option<Codec> {
    let mut header = [0; HEADER_LEN];
    archive
        .open_entry(entry)
        .ok()?
        .read_exact(&mut header)
        .ok()?;
    if header.get(12..16) != Some(b"fmt ") {
        return None;
    }
    let tag = [header[20], header[21]];
    Some(Codec::from_tag(match &header[..4] {
        b"RIFX" => u16::from_be_bytes(tag),
        _ => u16::from_le_bytes(tag),
    }))
}

fn decode(archive: &Archive, entry: &Entry) -> Result<(), String> {
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .map_err(|error| error.to_string())?
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let wem = Wem::parse(bytes.as_slice()).map_err(|error| error.to_string())?;
    let mut wav = Vec::new();
    let report = wem.write_wav(&mut wav).map_err(|error| error.to_string())?;
    if Some(report.sample_count) != wem.sample_count() || report.size != wav.len() as u64 {
        return Err(format!(
            "{report:?} disagrees with {:?} frames and {} bytes",
            wem.sample_count(),
            wav.len()
        ));
    }
    Ok(())
}
