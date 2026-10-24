// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! ADPCM WEMs encoded from the known signal decode back to it within tolerance.

use p4kx_audio::{AudioError, Wem};
use p4kx_testkit::{TestWem, test_signal};

type TestResult = Result<(), AudioError>;

const MAX_ERROR: i32 = 1_600;
const RMS_ERROR: f64 = 200.0;

fn decode(wem: &TestWem) -> Result<(Vec<i16>, u64), AudioError> {
    let mut wav = Vec::new();
    let report = Wem::parse(wem.bytes())?.write_wav(&mut wav)?;
    assert_eq!(report.size, wav.len() as u64, "the reported size");
    let data_len = report.sample_count * 2 * u64::from(report.channels);
    let samples = wav
        .get(
            wav.len()
                .saturating_sub(usize::try_from(data_len).unwrap_or(0))..,
        )
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(i16::from_le_bytes)
        .collect();
    Ok((samples, report.sample_count))
}

/// Returns the largest and the root-mean-square difference between the decoded samples and the
/// signal.
fn errors(decoded: &[i16], signal: &[i16]) -> (i32, f64) {
    let differences = || {
        decoded
            .iter()
            .zip(signal)
            .map(|(&decoded, &expected)| (i32::from(decoded) - i32::from(expected)).abs())
    };
    let squares: f64 = differences().map(|error| f64::from(error).powi(2)).sum();
    let count = f64::from(u32::try_from(signal.len()).unwrap_or(u32::MAX));
    (differences().max().unwrap_or(0), (squares / count).sqrt())
}

fn assert_close(decoded: &[i16], signal: &[i16], case: &str) {
    let (max, rms) = errors(decoded, signal);
    assert!(
        max <= MAX_ERROR && rms <= RMS_ERROR,
        "{case}: largest error {max}, RMS {rms:.1}"
    );
}

#[test]
fn ima_adpcm_decodes_the_known_signal_for_one_two_and_six_channels() -> TestResult {
    for channels in [1, 2, 6] {
        let signal = test_signal(4_800, channels);
        let wem = TestWem::ima_adpcm(channels, 48_000, &signal);

        let (decoded, frames) = decode(&wem)?;

        assert_eq!(
            frames,
            4_800_u64.div_ceil(64) * 64,
            "{channels}: whole blocks"
        );
        assert_eq!(
            decoded.len() as u64,
            frames * u64::from(channels),
            "{channels}: samples"
        );
        assert_close(&decoded, &signal, &format!("IMA × {channels}"));
    }
    Ok(())
}

#[test]
fn ptadpcm_decodes_the_known_signal_for_both_frame_sizes() -> TestResult {
    for (channels, frame_len, frame_samples) in [(1, 0x24, 64), (2, 0x24, 64), (6, 0x104, 512)] {
        let signal = test_signal(4_800, channels);
        let wem = TestWem::ptadpcm(channels, 48_000, frame_len, &signal);

        let (decoded, frames) = decode(&wem)?;

        assert_eq!(
            frames,
            4_800_u64.div_ceil(frame_samples) * frame_samples,
            "{channels} × {frame_len:#x}: whole frames"
        );
        assert_close(
            &decoded,
            &signal,
            &format!("PTADPCM {channels} × {frame_len:#x}"),
        );
    }
    Ok(())
}

#[test]
fn sounds_longer_than_one_batch_decode_every_block_in_order() -> TestResult {
    let signal = test_signal(5_120, 2);
    let repeats = 60;
    let expected = signal.repeat(repeats);
    for wem in [
        TestWem::ima_adpcm(2, 48_000, &signal),
        TestWem::ptadpcm(2, 48_000, 0x104, &signal),
    ] {
        let one = Wem::parse(wem.bytes())?.data().to_vec();
        let wem = wem.data(one.repeat(repeats));

        let (decoded, frames) = decode(&wem)?;

        assert_eq!(frames, 5_120 * 60, "every block");
        assert_close(&decoded, &expected, "1.2 MB of PCM");
    }
    Ok(())
}
