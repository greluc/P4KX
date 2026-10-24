// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The cost of validating a whole Wwise Vorbis stream instead of its first packets (ADR-0011):
//! the time to rebuild an Ogg against the time to decode all of it, and whether any rebuilt stream
//! fails after its first packets.

use std::io::{Cursor, Read};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use lewton::inside_ogg::OggStreamReader;
use p4kx_extraction::{Archive, Codec, Entry, Wem};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::{TestResult, report};

const SAMPLE: usize = 6_000;

#[derive(Default)]
struct Costs {
    rebuild_nanos: AtomicU64,
    decode_nanos: AtomicU64,
    bytes: AtomicU64,
}

fn elapsed(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn check(archive: &Archive, entry: &Entry, costs: &Costs) -> Result<bool, String> {
    let fail = |error: &dyn std::fmt::Display| format!("{}: {error}", entry.path());
    let mut data = Vec::new();
    archive
        .open_entry(entry)
        .map_err(|error| fail(&error))?
        .read_to_end(&mut data)
        .map_err(|error| fail(&error))?;
    let Ok(wem) = Wem::parse(data.as_slice()) else {
        return Ok(false);
    };
    if wem.codec() != Codec::Vorbis {
        return Ok(false);
    }
    let started = Instant::now();
    let mut ogg = Vec::new();
    wem.write_ogg(&mut ogg).map_err(|error| fail(&error))?;
    costs
        .rebuild_nanos
        .fetch_add(elapsed(started), Ordering::Relaxed);
    let started = Instant::now();
    let mut reader = OggStreamReader::new(Cursor::new(ogg)).map_err(|error| fail(&error))?;
    while reader
        .read_dec_packet_itl()
        .map_err(|error| fail(&error))?
        .is_some()
    {}
    costs
        .decode_nanos
        .fetch_add(elapsed(started), Ordering::Relaxed);
    costs.bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
    Ok(true)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_sampled_vorbis_stream_decodes_to_the_end_and_the_cost_is_measured() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let wems: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().as_bytes();
            path.len() > 4
                && path
                    .get(path.len() - 4..)
                    .is_some_and(|tail| tail.eq_ignore_ascii_case(b".wem"))
        })
        .collect();
    let stride = (wems.len() / SAMPLE).max(1);
    let costs = Costs::default();
    let results: Vec<Result<bool, String>> = wems
        .par_iter()
        .step_by(stride)
        .map(|entry| check(&archive, entry, &costs))
        .collect();
    let failures: Vec<&String> = results.iter().filter_map(|r| r.as_ref().err()).collect();
    let checked = results.iter().filter(|r| matches!(r, Ok(true))).count();
    report(&format!(
        "{checked} Vorbis WEMs ({} MiB): rebuild with 16 checked packets {} ms, full decode {} ms \
         (CPU time over all threads), {} failures",
        costs.bytes.load(Ordering::Relaxed) >> 20,
        costs.rebuild_nanos.load(Ordering::Relaxed) / 1_000_000,
        costs.decode_nanos.load(Ordering::Relaxed) / 1_000_000,
        failures.len()
    ));
    assert!(
        failures.is_empty(),
        "{:#?}",
        failures.iter().take(20).collect::<Vec<_>>()
    );
    Ok(())
}
