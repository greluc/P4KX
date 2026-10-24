// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The loops of the loose WEMs of a real Archive survive WAV and Ogg conversion.

use std::error::Error;
use std::io::{self, Read};

use p4kx_extraction::{Archive, AudioError, Codec, Wem};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const MAX_WEM_LEN: u64 = 64 << 20;
const CONVERTED: usize = 200;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn looping_wems_keep_their_loop_in_wav_and_ogg() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let (mut parsed, mut looping, mut converted, mut single_frame) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    let mut bytes = Vec::new();
    for entry in archive.entries().iter().filter(|entry| {
        entry.path().to_ascii_lowercase().ends_with(".wem")
            && entry.uncompressed_size() <= MAX_WEM_LEN
    }) {
        bytes.clear();
        archive.open_entry(entry)?.read_to_end(&mut bytes)?;
        let Ok(wem) = Wem::parse(bytes.as_slice()) else {
            continue;
        };
        parsed += 1;
        let Some(points) = wem.loop_points() else {
            continue;
        };
        looping += 1;
        single_frame += usize::from(points.frames() <= 1);
        assert!(
            points.fits(wem.sample_count().unwrap_or_default()),
            "{}: {points:?} within the sound",
            entry.path()
        );
        if converted >= CONVERTED {
            continue;
        }
        match wem.write_wav(&mut io::sink()) {
            Ok(report) => assert_eq!(
                report.loop_points,
                Some(points),
                "{}: the WAV keeps the loop",
                entry.path()
            ),
            Err(AudioError::UnsupportedCodec(_)) => {}
            Err(error) => return Err(format!("{}: {error}", entry.path()).into()),
        }
        if matches!(wem.codec(), Codec::Vorbis | Codec::Opus) {
            let report = wem.write_ogg(&mut io::sink())?;
            assert_eq!(
                report.loop_points,
                Some(points),
                "{}: the Ogg keeps the loop",
                entry.path()
            );
        }
        converted += 1;
    }
    assert!(parsed > 1_000, "{parsed} loose WEMs");
    assert!(looping > 1_000, "{looping} of {parsed} loose WEMs loop");
    assert_eq!(
        single_frame, 0,
        "{single_frame} of {looping} loops hold one frame or less"
    );
    Ok(())
}
