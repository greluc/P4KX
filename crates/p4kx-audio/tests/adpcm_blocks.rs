//! Partial blocks and block alignments that do not fit the channels of ADPCM WEMs.

use p4kx_audio::{AudioError, Wem};
use p4kx_testkit::{TestCodec, TestWem};

type TestResult = Result<(), AudioError>;

const IMA: TestCodec = TestCodec::Tag(0x0002);
const PTADPCM: TestCodec = TestCodec::Tag(0x8311);

#[test]
fn a_trailing_partial_ima_block_decodes_its_shorter_sub_blocks() -> TestResult {
    let wem = Wem::parse(
        TestWem::encoded(IMA, 2, 48_000, 0)
            .block_align(0x48)
            .data(vec![0; 0x48 + 2 * 0x10 + 1])
            .bytes(),
    )?;
    let mut wav = Vec::new();

    let report = wem.write_wav(&mut wav)?;

    assert_eq!(
        report.sample_count,
        64 + 24,
        "a 16-byte sub-block holds 24 samples"
    );
    assert_eq!(wem.sample_count(), Some(64 + 24), "the parsed count agrees");
    assert_eq!(wav.len(), 44 + 88 * 4, "the whole WAV");
    assert_eq!(report.size, wav.len() as u64, "the reported size");
    Ok(())
}

#[test]
fn a_trailing_partial_ptadpcm_frame_is_dropped() -> TestResult {
    let wem = Wem::parse(
        TestWem::encoded(PTADPCM, 1, 48_000, 0)
            .block_align(0x104)
            .data(vec![0; 0x104 * 2 + 0x80])
            .bytes(),
    )?;
    let mut wav = Vec::new();

    let report = wem.write_wav(&mut wav)?;

    assert_eq!(
        report.sample_count,
        2 * 512,
        "512 samples per 0x104-byte frame"
    );
    assert_eq!(wem.sample_count(), Some(2 * 512), "the parsed count agrees");
    assert_eq!(wav.len(), 44 + 1024 * 2, "the whole WAV");
    Ok(())
}

#[test]
fn ima_data_shorter_than_one_header_per_channel_is_an_empty_wav() -> TestResult {
    let wem = Wem::parse(
        TestWem::encoded(IMA, 2, 48_000, 0)
            .block_align(0x48)
            .data(vec![0; 9])
            .bytes(),
    )?;
    let mut wav = Vec::new();

    let report = wem.write_wav(&mut wav)?;

    assert_eq!(report.sample_count, 0, "no frames");
    assert_eq!(wav.len(), 44, "the header only");
    Ok(())
}

#[test]
fn block_alignments_that_do_not_fit_the_channels_are_invalid() -> TestResult {
    let cases = [
        (IMA, 2, 0x47, "not a multiple of the channels"),
        (IMA, 3, 0x4A, "not a multiple of the channels"),
        (IMA, 1, 4, "the header only"),
        (IMA, 1, 0, "zero"),
        (PTADPCM, 1, 5, "the header only"),
        (PTADPCM, 2, 0x49, "not a multiple of the channels"),
    ];
    for (codec, channels, block_align, why) in cases {
        let wem = Wem::parse(
            TestWem::encoded(codec, channels, 48_000, 0)
                .block_align(block_align)
                .data(vec![0x5A; 0x200])
                .bytes(),
        )?;
        let mut wav = Vec::new();

        let error = wem.write_wav(&mut wav).err();

        assert_eq!(
            wem.sample_count(),
            None,
            "{block_align:#x}, {why}: no sample count"
        );
        assert!(
            matches!(error, Some(AudioError::InvalidFormat(_))),
            "{block_align:#x}, {why}: {error:?}"
        );
        assert!(
            wav.is_empty(),
            "{block_align:#x}, {why}: nothing is written"
        );
    }
    Ok(())
}

#[test]
fn every_byte_pattern_decodes_without_panicking() -> TestResult {
    for codec in [IMA, PTADPCM] {
        for channels in 1..=6 {
            let data: Vec<u8> = (0..0x24 * u32::from(channels) * 5 + 7)
                .map(|index| u8::try_from(index.wrapping_mul(2_654_435_761) >> 24).unwrap_or(0))
                .collect();
            let wem = Wem::parse(
                TestWem::encoded(codec, channels, 48_000, 0)
                    .block_align(0x24 * channels)
                    .data(data)
                    .bytes(),
            )?;
            let mut wav = Vec::new();

            let report = wem.write_wav(&mut wav)?;

            assert_eq!(
                report.size,
                wav.len() as u64,
                "{codec:?} × {channels}: size"
            );
            assert_eq!(
                Some(report.sample_count),
                wem.sample_count(),
                "{codec:?} × {channels}: frames"
            );
        }
    }
    Ok(())
}
