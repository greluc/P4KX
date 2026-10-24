// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Opus channel layouts beyond the Vorbis surround orders, and the `OpusHead` output gain.

use std::io::Cursor;

use crate::opus_signal::{snr, wav_samples};
use opus_pure::OggOpusReader;
use p4kx_audio::Wem;
use p4kx_testkit::{TestOpus, TestOpusFraming};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FRAMES: u32 = 9_600;
const MIN_SNR_DB: f64 = 20.0;

#[test]
fn wwise_mapping_255_keeps_one_stream_per_channel_in_channel_order() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Wwise, 6, FRAMES).mapping(255);
    let bytes = sound.wem()?.bytes();
    let wem = Wem::parse(bytes.as_slice())?;

    let mut ogg = Vec::new();
    wem.write_ogg(&mut ogg)?;
    let reader = OggOpusReader::new(Cursor::new(ogg))?;
    let head = reader.head();
    assert_eq!(
        (head.mapping_family, head.stream_count, head.coupled_count),
        (255, 6, 0),
        "family 255 with six mono streams"
    );
    assert_eq!(head.channel_mapping, [0, 1, 2, 3, 4, 5], "identity mapping");

    let mut wav = Vec::new();
    wem.write_wav(&mut wav)?;
    let samples = wav_samples(&wav, 6);
    let signal = sound.signal();
    for channel in 0..6 {
        let snr = snr(&signal, &samples, 6, channel);
        assert!(
            snr > MIN_SNR_DB,
            "WAV channel {channel} is encoded channel {channel}: {snr:.1} dB"
        );
    }
    Ok(())
}

#[test]
fn a_plain_channel_mask_without_the_wwise_config_still_decodes_stereo() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Wwise, 2, FRAMES).raw_layout(0x3);
    let bytes = sound.wem()?.bytes();
    let wem = Wem::parse(bytes.as_slice())?;
    let mut wav = Vec::new();
    wem.write_wav(&mut wav)?;
    let samples = wav_samples(&wav, 2);
    let signal = sound.signal();
    for channel in 0..2 {
        let snr = snr(&signal, &samples, 2, channel);
        assert!(snr > MIN_SNR_DB, "channel {channel}: {snr:.1} dB");
    }
    Ok(())
}

#[test]
fn the_output_gain_of_an_embedded_opus_head_scales_the_wav() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Ogg, 1, FRAMES).output_gain(-1541);
    let bytes = sound.wem()?.bytes();
    let mut wav = Vec::new();
    Wem::parse(bytes.as_slice())?.write_wav(&mut wav)?;
    let half: Vec<i16> = sound.signal().iter().map(|sample| sample / 2).collect();
    let snr = snr(&half, &wav_samples(&wav, 1), 1, 0);
    assert!(
        snr > MIN_SNR_DB,
        "half the amplitude at -6.02 dB: {snr:.1} dB"
    );
    Ok(())
}

#[test]
fn the_testkit_refuses_seven_channels_without_a_wwise_layout() {
    let sound = TestOpus::new(TestOpusFraming::Wwise, 7, FRAMES);
    assert!(
        sound.channel_mask().is_none(),
        "no Wwise mask for 7 channels"
    );
    assert!(sound.wem().is_err(), "no WEM for 7 channels with mapping 1");
}
