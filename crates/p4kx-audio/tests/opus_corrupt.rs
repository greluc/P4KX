//! Corrupt Opus WEMs give typed errors and never panic.

use p4kx_audio::{AudioError, OpusError, Wem};
use p4kx_testkit::{TestOpus, TestOpusFraming};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FRAMES: u32 = 4_800;
const SWEEP_FRAMES: u32 = 1_000;

fn opus_error(bytes: &[u8]) -> Result<OpusError, Box<dyn std::error::Error>> {
    let wem = Wem::parse(bytes)?;
    let mut ogg = Vec::new();
    let ogg_error = wem.write_ogg(&mut ogg).err();
    let mut wav = Vec::new();
    match (ogg_error, wem.write_wav(&mut wav)) {
        (Some(AudioError::InvalidOpus(ogg_error)), Err(AudioError::InvalidOpus(wav_error))) => {
            assert_eq!(ogg_error, wav_error, "Ogg and WAV agree");
            assert!(ogg.is_empty() && wav.is_empty(), "nothing written");
            Ok(ogg_error)
        }
        other => Err(format!("no Opus error: {other:?}").into()),
    }
}

fn find(bytes: &[u8], needle: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .ok_or("field not found")?)
}

#[test]
fn a_seek_chunk_shorter_than_the_packet_count_is_an_invalid_header() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Wwise, 1, FRAMES);
    let mut bytes = sound.wem()?.bytes();
    let count = u32::try_from(sound.packets()?.packets.len())?;
    let at = find(&bytes, b"fmt ")? + 8 + 0x1C;
    bytes
        .get_mut(at..at + 4)
        .ok_or("count")?
        .copy_from_slice(&(count + 1).to_le_bytes());
    assert_eq!(
        opus_error(&bytes)?,
        OpusError::InvalidHeader(
            "the seek chunk holds fewer packet sizes than the fmt chunk counts"
        )
    );
    Ok(())
}

#[test]
fn a_packet_size_beyond_the_data_is_out_of_bounds() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Wwise, 1, FRAMES);
    let mut bytes = sound.wem()?.bytes();
    let at = find(&bytes, b"seek")? + 8 + 2;
    bytes
        .get_mut(at..at + 2)
        .ok_or("size")?
        .copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(
        matches!(
            opus_error(&bytes)?,
            OpusError::PacketOutOfBounds { index: 1 }
        ),
        "packet 1"
    );
    Ok(())
}

#[test]
fn an_unknown_wwise_opus_version_is_an_invalid_header() -> TestResult {
    let mut bytes = TestOpus::new(TestOpusFraming::Wwise, 2, FRAMES)
        .wem()?
        .bytes();
    let at = find(&bytes, b"fmt ")? + 8 + 0x22;
    *bytes.get_mut(at).ok_or("version")? = 2;
    assert_eq!(
        opus_error(&bytes)?,
        OpusError::InvalidHeader("unknown Wwise Opus version")
    );
    Ok(())
}

#[test]
fn an_embedded_ogg_page_with_a_wrong_crc_is_rejected() -> TestResult {
    let mut bytes = TestOpus::new(TestOpusFraming::Ogg, 2, FRAMES)
        .wem()?
        .bytes();
    let first_page = find(&bytes, b"OggS")?;
    let second_page = first_page + 4 + find(bytes.get(first_page + 4..).ok_or("rest")?, b"OggS")?;
    *bytes.get_mut(second_page + 30).ok_or("tags")? ^= 0x20;
    assert_eq!(
        opus_error(&bytes)?,
        OpusError::InvalidOgg("a page's CRC does not match")
    );
    Ok(())
}

#[test]
fn a_garbage_toc_is_an_invalid_packet() -> TestResult {
    let sound = TestOpus::new(TestOpusFraming::Switch, 1, FRAMES);
    let mut bytes = sound.wem()?.bytes();
    let first_packet = find(&bytes, b"data")? + 8 + 4 + 8;
    *bytes.get_mut(first_packet).ok_or("toc")? = 0x03;
    *bytes.get_mut(first_packet + 1).ok_or("count")? = 0;
    assert_eq!(
        opus_error(&bytes)?,
        OpusError::InvalidPacket {
            index: 0,
            reason: "the packet holds no frame"
        }
    );
    Ok(())
}

#[test]
fn opus_without_packets_holds_no_samples() -> TestResult {
    let bytes = TestOpus::new(TestOpusFraming::Wwise, 1, FRAMES)
        .wem()?
        .data(Vec::new())
        .bytes();
    let error = opus_error(&bytes)?;
    assert!(
        matches!(
            error,
            OpusError::PacketOutOfBounds { index: 0 } | OpusError::NoSamples { .. }
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn flipped_and_truncated_bytes_never_panic() -> TestResult {
    for (framing, channels) in [
        (TestOpusFraming::Wwise, 6),
        (TestOpusFraming::Wwise, 2),
        (TestOpusFraming::Ogg, 2),
        (TestOpusFraming::Switch, 2),
    ] {
        let bytes = TestOpus::new(framing, channels, SWEEP_FRAMES)
            .wem()?
            .bytes();
        let mut mutated = bytes.clone();
        for position in (0..bytes.len()).step_by(5) {
            for mask in [0x01, 0xFF] {
                mutated.copy_from_slice(&bytes);
                if let Some(byte) = mutated.get_mut(position) {
                    *byte ^= mask;
                }
                convert(&mutated);
            }
        }
        for len in (0..bytes.len()).step_by(7) {
            convert(bytes.get(..len).unwrap_or_default());
        }
    }
    Ok(())
}

fn convert(bytes: &[u8]) {
    if let Ok(wem) = Wem::parse(bytes) {
        let mut out = Vec::new();
        let ogg = wem.write_ogg(&mut out).map(|report| report.size);
        out.clear();
        let wav = wem.write_wav(&mut out).map(|report| report.size);
        if let Ok(size) = wav {
            assert_eq!(size, out.len() as u64, "reported WAV size");
        }
        drop(ogg);
    }
}
