//! Opus WEMs of every framing: WAV decoding against the known signal and Ogg Opus repackaging.

mod pages;

use std::io::Cursor;

use opus_pure::{OggOpusReader, OpusMSDecoder};
use p4kx_audio::{AudioError, Codec, OpusError, Wem};
use p4kx_testkit::{TestOpus, TestOpusFraming};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FRAMES: u32 = 9_600;
const MIN_SNR_DB: f64 = 20.0;

/// Returns the signal-to-noise ratio in dB of channel `channel` of `decoded` against `signal`.
fn snr(signal: &[i16], decoded: &[i16], channels: usize, channel: usize) -> f64 {
    let (mut power, mut noise) = (0.0_f64, 0.0_f64);
    for (expected, got) in signal
        .iter()
        .skip(channel)
        .step_by(channels)
        .zip(decoded.iter().skip(channel).step_by(channels))
    {
        power += f64::from(*expected).powi(2);
        noise += (f64::from(*expected) - f64::from(*got)).powi(2);
    }
    10.0 * (power / noise.max(1.0)).log10()
}

fn wav_samples(wav: &[u8], channels: u16) -> Vec<i16> {
    let header_len = if channels > 2 { 68 } else { 44 };
    wav.get(header_len..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}

fn assert_wav_matches(sound: &TestOpus, channels: u16) -> TestResult {
    let bytes = sound.wem().bytes();
    let wem = Wem::parse(bytes.as_slice())?;
    assert_eq!(wem.codec(), Codec::Opus, "codec");
    assert_eq!(wem.sample_rate(), 48_000, "rate");
    let mut wav = Vec::new();
    let report = wem.write_wav(&mut wav)?;
    assert_eq!(report.size, wav.len() as u64, "reported size");
    assert_eq!(report.sample_count, u64::from(FRAMES), "frames");
    assert_eq!(report.sample_rate, 48_000, "WAV rate");
    let samples = wav_samples(&wav, channels);
    assert_eq!(
        samples.len(),
        FRAMES as usize * usize::from(channels),
        "samples"
    );
    let signal = sound.signal();
    for channel in 0..usize::from(channels) {
        let snr = snr(&signal, &samples, usize::from(channels), channel);
        assert!(snr > MIN_SNR_DB, "channel {channel}: {snr:.1} dB");
    }
    Ok(())
}

#[test]
fn wwise_opus_decodes_to_the_known_signal_in_mono_stereo_and_surround() -> TestResult {
    for channels in [1, 2, 4, 6, 8] {
        assert_wav_matches(
            &TestOpus::new(TestOpusFraming::Wwise, channels, FRAMES),
            channels,
        )?;
    }
    Ok(())
}

#[test]
fn ogg_and_switch_framed_opus_decode_to_the_known_signal() -> TestResult {
    assert_wav_matches(&TestOpus::new(TestOpusFraming::Ogg, 2, FRAMES), 2)?;
    assert_wav_matches(&TestOpus::new(TestOpusFraming::Ogg, 6, FRAMES), 6)?;
    assert_wav_matches(&TestOpus::new(TestOpusFraming::Switch, 1, FRAMES), 1)?;
    assert_wav_matches(&TestOpus::new(TestOpusFraming::Switch, 2, FRAMES), 2)
}

/// Checks the pages and packets of `ogg` against `sound` and that it plays the signal.
fn assert_playable_ogg(sound: &TestOpus, ogg: &[u8]) -> TestResult {
    let pages = pages::pages(ogg)?;
    assert!(
        pages.first().is_some_and(pages::Page::first),
        "BOS on the first page"
    );
    assert!(
        pages.last().is_some_and(pages::Page::last),
        "EOS on the last page"
    );
    assert!(
        pages.iter().skip(1).all(|page| !page.first()),
        "BOS only once"
    );
    assert!(
        pages.iter().take(2).all(|page| !page.continued()),
        "whole header pages"
    );
    let packets = pages::packets(&pages);
    let encoded = sound.packets();
    assert_eq!(
        packets.len(),
        2 + encoded.packets.len(),
        "header and audio packets"
    );
    assert_eq!(
        packets.get(2).map(|packet| &packet.0),
        encoded.packets.first(),
        "packet bytes"
    );
    let pre_skip = encoded.pre_skip;
    let last = pages.last().map(|page| page.granule);
    assert_eq!(
        last,
        Some(u64::from(pre_skip) + u64::from(FRAMES)),
        "end granule"
    );
    assert_plays_the_signal(sound, ogg, pre_skip)
}

/// Decodes `ogg` as a player would and compares every channel with the signal of `sound`.
fn assert_plays_the_signal(sound: &TestOpus, ogg: &[u8], pre_skip: u16) -> TestResult {
    let mut reader = OggOpusReader::new(Cursor::new(ogg.to_vec()))?;
    let head = reader.head().clone();
    assert_eq!(head.pre_skip, pre_skip, "pre-skip");
    let channels = usize::from(head.channel_count);
    let mut decoder = OpusMSDecoder::new(48_000, channels, head.mapping_family)?;
    let mut samples = Vec::new();
    let mut out = vec![0_i16; 5_760 * channels];
    while let Some(packet) = reader.read_packet()? {
        let frames = decoder.decode_s16(&packet.data, 5_760, &mut out)?;
        samples.extend_from_slice(out.get(..frames * channels).unwrap_or_default());
    }
    let played = samples
        .get(usize::from(pre_skip) * channels..)
        .unwrap_or_default();
    let signal = sound.signal();
    let order: &[usize] = if channels == 6 {
        &[0, 2, 1, 4, 5, 3]
    } else {
        &[0, 1]
    };
    for (vorbis, &wave) in order.iter().enumerate().take(channels) {
        let wave_signal: Vec<i16> = signal
            .iter()
            .skip(wave)
            .step_by(channels)
            .copied()
            .collect();
        let vorbis_played: Vec<i16> = played
            .iter()
            .skip(vorbis)
            .step_by(channels)
            .copied()
            .collect();
        let snr = snr(&wave_signal, &vorbis_played, 1, 0);
        assert!(snr > MIN_SNR_DB, "Ogg channel {vorbis}: {snr:.1} dB");
    }
    Ok(())
}

#[test]
fn wwise_opus_becomes_a_playable_ogg_opus_with_its_pre_skip_and_length() -> TestResult {
    for (framing, channels) in [
        (TestOpusFraming::Wwise, 1),
        (TestOpusFraming::Wwise, 2),
        (TestOpusFraming::Ogg, 2),
        (TestOpusFraming::Switch, 2),
    ] {
        let sound = TestOpus::new(framing, channels, FRAMES);
        let bytes = sound.wem().bytes();
        let wem = Wem::parse(bytes.as_slice())?;
        let mut ogg = Vec::new();
        let report = wem.write_ogg(&mut ogg)?;
        assert_eq!(report.codec, Codec::Opus, "codec");
        assert_eq!(report.codebooks, None, "no codebooks");
        assert_eq!(report.size, ogg.len() as u64, "size");
        assert_eq!(report.sample_count, u64::from(FRAMES), "frames");
        assert_playable_ogg(&sound, &ogg)?;
    }
    Ok(())
}

#[test]
fn surround_opus_gets_the_vorbis_channel_mapping_of_the_wwise_layout() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Wwise, 6, FRAMES);
    let bytes = sound.wem().bytes();
    let mut ogg = Vec::new();
    Wem::parse(bytes.as_slice())?.write_ogg(&mut ogg)?;
    let reader = OggOpusReader::new(Cursor::new(ogg.clone()))?;
    let head = reader.head();
    assert_eq!(
        (
            head.channel_count,
            head.mapping_family,
            head.stream_count,
            head.coupled_count
        ),
        (6, 1, 4, 2),
        "OpusHead"
    );
    assert_eq!(head.channel_mapping, [0, 4, 1, 2, 3, 5], "mapping");
    assert_playable_ogg(&sound, &ogg)
}

#[test]
fn a_shorter_declared_sample_count_trims_wav_and_ogg_alike() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Wwise, 1, FRAMES).declared_sample_count(5_000);
    let bytes = sound.wem().bytes();
    let wem = Wem::parse(bytes.as_slice())?;
    assert_eq!(wem.sample_count(), Some(5_000), "declared");
    let wav = wem.write_wav(&mut Vec::new())?;
    let mut ogg = Vec::new();
    let report = wem.write_ogg(&mut ogg)?;
    assert_eq!(
        (wav.sample_count, report.sample_count),
        (5_000, 5_000),
        "trimmed"
    );
    let pages = pages::pages(&ogg)?;
    let granules: Vec<u64> = pages
        .iter()
        .map(|page| page.granule)
        .filter(|&g| g != u64::MAX)
        .collect();
    assert!(
        granules.is_sorted(),
        "granules never decrease: {granules:?}"
    );
    let end = u64::from(sound.packets().pre_skip) + 5_000;
    assert_eq!(granules.last(), Some(&end), "end granule");
    Ok(())
}

#[test]
fn a_switch_opus_at_another_rate_counts_its_samples_from_the_packets() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Switch, 1, FRAMES);
    let mut bytes = sound.wem().bytes();
    let rate = bytes
        .windows(4)
        .position(|window| window == 48_000_u32.to_le_bytes())
        .ok_or("rate field")?;
    bytes
        .get_mut(rate..rate + 4)
        .ok_or("rate")?
        .copy_from_slice(&24_000_u32.to_le_bytes());
    let wem = Wem::parse(bytes.as_slice())?;
    assert_eq!(
        (wem.sample_rate(), wem.sample_count()),
        (48_000, None),
        "header"
    );
    let report = wem.write_wav(&mut Vec::new())?;
    let packets = sound.packets();
    let expected = packets.packets.len() as u64 * 960 - u64::from(packets.pre_skip);
    assert_eq!(
        report.sample_count, expected,
        "every packet but the first's delay"
    );
    Ok(())
}

#[test]
fn a_layout_without_an_opus_mapping_is_a_typed_error() -> TestResult {
    let bytes = TestOpus::new(TestOpusFraming::Wwise, 6, FRAMES)
        .mapping(0)
        .wem()
        .bytes();
    let wem = Wem::parse(bytes.as_slice())?;
    let expected = OpusError::UnsupportedLayout {
        channels: 6,
        mapping: 0,
    };
    assert!(
        matches!(wem.write_wav(&mut Vec::new()), Err(AudioError::InvalidOpus(ref e)) if *e == expected),
        "WAV"
    );
    let mut ogg = Vec::new();
    assert!(
        matches!(wem.write_ogg(&mut ogg), Err(AudioError::InvalidOpus(ref e)) if *e == expected),
        "Ogg"
    );
    assert!(ogg.is_empty(), "nothing written");
    Ok(())
}
