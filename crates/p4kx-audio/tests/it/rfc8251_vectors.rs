//! The decode path against the official RFC 8251 Opus decoder test vectors, kept outside the
//! repository.

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use p4kx_audio::Wem;
use p4kx_testkit::{TestOpusPackets, wwise_opus_wem};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const VECTORS: usize = 12;
const MAX_LSB: u16 = 1;
const MIN_SNR_DB: f64 = 90.0;
const STEREO_LAYOUT: u32 = (0x3 << 12) | (1 << 8) | 2;

/// Splits an `opus_demo` bit stream into its packets: each after a big-endian length and the
/// encoder's final range.
fn packets(bit: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut packets = Vec::new();
    let mut rest = bit;
    while let Some((header, tail)) = rest.split_first_chunk::<8>() {
        let len = header
            .first_chunk::<4>()
            .map(|len| u32::from_be_bytes(*len))
            .and_then(|len| usize::try_from(len).ok())
            .ok_or("a packet length is unreadable")?;
        let packet = tail.get(..len).ok_or("a packet runs past the file")?;
        packets.push(packet.to_vec());
        rest = tail.get(len..).unwrap_or_default();
    }
    Ok(packets)
}

fn samples(bytes: &[u8]) -> Vec<i16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}

/// Returns the largest difference and the signal-to-noise ratio in dB of `decoded` against
/// `reference`, or `None` when their lengths differ.
fn compare(reference: &[i16], decoded: &[i16]) -> Option<(u16, f64)> {
    if reference.len() != decoded.len() {
        return None;
    }
    let (mut power, mut noise, mut worst) = (0.0_f64, 0.0_f64, 0_u16);
    for (&expected, &got) in reference.iter().zip(decoded) {
        power += f64::from(expected).powi(2);
        noise += (f64::from(expected) - f64::from(got)).powi(2);
        worst = worst.max(expected.abs_diff(got));
    }
    let snr = if noise == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (power / noise).log10()
    };
    Some((worst, snr))
}

#[test]
#[ignore = "RFC 8251 vectors: set P4KX_OPUS_VECTORS and pass --ignored"]
#[expect(clippy::print_stderr, reason = "the skip note of an optional test")]
fn wav_decoding_matches_the_rfc_8251_test_vectors() -> TestResult {
    let Some(folder) = env::var_os("P4KX_OPUS_VECTORS").map(PathBuf::from) else {
        eprintln!("skipped: set P4KX_OPUS_VECTORS to the folder of the RFC 8251 test vectors");
        return Ok(());
    };
    let mut failures = Vec::new();
    for vector in 1..=VECTORS {
        let name = |suffix: &str| folder.join(format!("testvector{vector:02}{suffix}"));
        let encoded = TestOpusPackets {
            packets: packets(&fs::read(name(".bit"))?)?,
            pre_skip: 0,
        };
        let wem = wwise_opus_wem(2, &encoded, u32::MAX, STEREO_LAYOUT, 1).bytes();
        let mut wav = Vec::new();
        Wem::parse(wem.as_slice())?.write_wav(&mut wav)?;
        let decoded = samples(wav.get(44..).unwrap_or_default());
        let results = [".dec", "m.dec"].map(|suffix| {
            fs::read(name(suffix)).map(|reference| compare(&samples(&reference), &decoded))
        });
        let [first, second] = results;
        let (first, second) = (first?, second?);
        let passes = |result: Option<(u16, f64)>| {
            result.is_some_and(|(worst, snr)| worst <= MAX_LSB || snr >= MIN_SNR_DB)
        };
        writeln!(
            std::io::stderr(),
            "vector {vector:02}: {} packets, .dec {first:?}, m.dec {second:?}",
            encoded.packets.len()
        )?;
        if !passes(first) && !passes(second) {
            failures.push(vector);
        }
    }
    assert!(
        failures.is_empty(),
        "vectors off the reference: {failures:?}"
    );
    Ok(())
}
