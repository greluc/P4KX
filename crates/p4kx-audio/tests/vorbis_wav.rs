//! Decoding Wwise Vorbis WEMs to PCM WAV: the known signal, the length, the format and corrupt
//! input.

use p4kx_audio::{AudioError, VorbisError, WavReport, Wem};
use p4kx_testkit::{TestCodebook, TestVorbis};

const HOP: usize = 128;
const BIN: u8 = 10;
const TOLERANCE: f64 = 2.0 / 32_768.0;

fn convert(bytes: Vec<u8>) -> Result<(Vec<u8>, WavReport), AudioError> {
    let wem = Wem::parse(bytes)?;
    let mut out = Vec::new();
    wem.write_wav(&mut out).map(|report| (out, report))
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// Returns the samples of the `data` chunk, the last chunk of the WAV.
fn samples(wav: &[u8]) -> Vec<i16> {
    let data = wav
        .windows(4)
        .position(|window| window == b"data")
        .map_or(wav.len(), |start| start + 8);
    wav.get(data..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(i16::from_le_bytes)
        .collect()
}

/// Returns how far channel `channel` of `samples` strays from the tone of spectral line `line`
/// in short blocks of 256 samples.
fn deviation(samples: &[i16], channels: usize, channel: usize, line: u8) -> f64 {
    let hop = TestVorbis::new(1, 48_000, &[]).tone_hop(line);
    samples
        .iter()
        .skip(channel)
        .step_by(channels)
        .zip(hop.iter().cycle())
        .map(|(&sample, &expected)| (f64::from(sample) / 32_768.0 - expected).abs())
        .fold(0.0, f64::max)
}
#[test]
fn a_vorbis_tone_decodes_to_the_known_signal() {
    let sound = TestVorbis::new(1, 48_000, &[false; 400]).tone(BIN);

    let (wav, report) = convert(sound.wem().bytes()).unwrap();

    let frames = 399 * 128;
    assert_eq!(
        report,
        WavReport {
            channels: 1,
            sample_rate: 48_000,
            sample_count: frames,
            size: 44 + 2 * frames,
        },
        "the report"
    );
    assert_eq!(wav.get(..4), Some(&b"RIFF"[..]), "a RIFF file");
    assert_eq!(u16_at(&wav, 20), Some(1), "PCM");
    assert_eq!(u16_at(&wav, 34), Some(16), "16 bits");
    assert_eq!(u32_at(&wav, 40), Some(2 * 399 * 128), "the data size");
    let decoded = samples(&wav);
    assert_eq!(decoded.len(), 399 * 128, "every frame");
    let worst = deviation(&decoded, 1, 0, BIN);
    assert!(
        worst <= TOLERANCE,
        "the samples are the known tone within {TOLERANCE}, off by {worst}"
    );
    assert!(
        decoded.iter().any(|&sample| sample.unsigned_abs() > 2_000),
        "the tone is audible"
    );
}

#[test]
fn stereo_is_interleaved_and_the_last_packet_trimmed_to_the_header_s_samples() {
    let sound = TestVorbis::new(2, 44_100, &[false; 100])
        .tone(3)
        .sample_count(12_600);

    let (wav, report) = convert(sound.wem().bytes()).unwrap();

    assert_eq!(report.channels, 2, "stereo");
    assert_eq!(
        report.sample_count, 12_600,
        "trimmed to the header's samples"
    );
    assert_eq!(u16_at(&wav, 22), Some(2), "stereo header");
    assert_eq!(u32_at(&wav, 24), Some(44_100), "sample rate");
    let decoded = samples(&wav);
    assert_eq!(decoded.len(), 25_200, "two samples a frame");
    for (channel, bin) in [(0, 3), (1, 4)] {
        let worst = deviation(&decoded, 2, channel, bin);
        assert!(worst <= TOLERANCE, "channel {channel}: off by {worst}");
    }
}

#[test]
fn surround_channels_keep_the_order_of_the_wem() {
    let sound = TestVorbis::new(6, 48_000, &[false; 50]).tone(2);

    let (wav, report) = convert(sound.wem().bytes()).unwrap();

    assert_eq!(report.channels, 6, "5.1");
    assert_eq!(u16_at(&wav, 20), Some(0xFFFE), "WAVE_FORMAT_EXTENSIBLE");
    let decoded = samples(&wav);
    assert_eq!(decoded.len(), 6 * 49 * HOP, "six samples a frame");
    for channel in 0..6 {
        let bin = 2 + u8::try_from(channel).unwrap();
        let worst = deviation(&decoded, 6, channel, bin);
        assert!(worst <= TOLERANCE, "channel {channel}: off by {worst}");
    }
}

#[test]
fn silent_library_codebook_sounds_decode_to_silence_of_the_right_length() {
    for sound in [
        TestVorbis::new(2, 48_000, &[false, true, true, false, true, false]),
        TestVorbis::new(1, 32_000, &[true, false, true]).big_endian(),
        TestVorbis::new(1, 32_000, &[true, false, true]).native_packets(),
        TestVorbis::new(1, 22_050, &[false, true, true, false]).codebook(TestCodebook::Library(97)),
    ] {
        let (wav, report) = convert(sound.wem().bytes()).unwrap();

        let frames = u64::from(sound.decoded_samples());
        assert_eq!(report.sample_count, frames, "{sound:?}: frames");
        assert_eq!(report.size, wav.len() as u64, "{sound:?}: size");
        assert!(
            samples(&wav).iter().all(|&sample| sample == 0),
            "{sound:?}: silence"
        );
    }
}

#[test]
fn a_vorbis_sound_without_usable_codebooks_is_a_typed_error() {
    let sound = TestVorbis::new(2, 48_000, &[false; 4]).codebook(TestCodebook::Library(598));

    let error = convert(sound.wem().bytes()).err();

    assert!(
        matches!(error, Some(AudioError::CodebooksRejected { .. })),
        "{error:?}"
    );
}

/// Returns the `data` chunk of `sound` with `packets` as its audio packets.
fn with_packets(sound: &TestVorbis, packets: &[Vec<u8>]) -> Vec<u8> {
    let setup = sound.setup();
    let mut data = u16::try_from(setup.len())
        .unwrap_or(u16::MAX)
        .to_le_bytes()
        .to_vec();
    data.extend_from_slice(&setup);
    for packet in packets {
        data.extend_from_slice(
            &u16::try_from(packet.len())
                .unwrap_or(u16::MAX)
                .to_le_bytes(),
        );
        data.extend_from_slice(packet);
    }
    data
}

#[test]
fn a_late_packet_the_decoder_rejects_is_a_typed_error_naming_its_offset() {
    let mut flags = [false; 24];
    flags[18] = true;
    flags[19] = true;
    let sound = TestVorbis::new(1, 48_000, &flags).native_packets();
    let mut packets = sound.packets();
    packets[19] = vec![0b0010];

    let error = convert(sound.wem().data(with_packets(&sound, &packets)).bytes()).err();

    let offset = 2 + sound.setup().len() + 19 * 3;
    assert!(
        matches!(
            &error,
            Some(AudioError::InvalidVorbis(VorbisError::Undecodable { offset: at, .. }))
                if *at == offset as u64
        ),
        "{error:?}"
    );
}
#[test]
fn damaged_vorbis_wems_never_panic() {
    let wem = TestVorbis::new(1, 48_000, &[false, true, false, true, false])
        .tone(2)
        .wem()
        .bytes();

    for len in 0..wem.len() {
        let _ignored = convert(wem[..len].to_vec());
    }
    for index in 0..wem.len() {
        for bit in 0..8 {
            let mut damaged = wem.clone();
            damaged[index] ^= 1 << bit;
            let _ignored = convert(damaged);
        }
    }
}
