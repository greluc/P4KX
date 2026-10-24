// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Format tag `0x0002` is IMA ADPCM, DSP ADPCM or PTADPCM by the layout of the `fmt` chunk.

use p4kx_audio::{AudioError, Codec, Wem};
use p4kx_testkit::{TestWem, test_signal};

type TestResult = Result<(), AudioError>;

const FMT_BODY: usize = 20;

fn fmt_wem(channels: u16, block_align: u16, extra: &[u8]) -> Vec<u8> {
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&2u16.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&48_000u32.to_le_bytes());
    fmt.extend_from_slice(&0u32.to_le_bytes());
    fmt.extend_from_slice(&block_align.to_le_bytes());
    fmt.extend_from_slice(&4u16.to_le_bytes());
    fmt.extend_from_slice(&u16::try_from(extra.len()).unwrap_or(u16::MAX).to_le_bytes());
    fmt.extend_from_slice(extra);
    let mut body = b"WAVEfmt ".to_vec();
    body.extend_from_slice(&u32::try_from(fmt.len()).unwrap_or(u32::MAX).to_le_bytes());
    body.extend_from_slice(&fmt);
    body.extend_from_slice(b"data");
    body.extend_from_slice(&64u32.to_le_bytes());
    body.extend_from_slice(&[0; 64]);
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[test]
fn tag_2_with_the_ima_layout_is_ima_adpcm() -> TestResult {
    for (channels, extra) in [(1u16, 2usize), (2, 2), (6, 0)] {
        let bytes = fmt_wem(channels, channels * 0x24, &vec![0; extra]);

        let wem = Wem::parse(bytes.as_slice())?;

        assert_eq!(wem.codec(), Codec::ImaAdpcm, "{channels} channels");
        assert_eq!(wem.format_tag(), 2, "the tag is kept");
    }
    Ok(())
}

#[test]
fn tag_2_with_dsp_coefficients_is_dsp_adpcm_and_unsupported() -> TestResult {
    for channels in [1u16, 2] {
        let extra = vec![0; 0x0C + usize::from(channels) * 0x2E];
        let bytes = fmt_wem(channels, 8, &extra);

        let wem = Wem::parse(bytes.as_slice())?;

        assert_eq!(wem.codec(), Codec::Dsp, "{channels} channels");
        assert!(
            matches!(
                wem.write_wav(&mut Vec::new()),
                Err(AudioError::UnsupportedCodec(Codec::Dsp))
            ),
            "no decoder"
        );
    }
    Ok(())
}

#[test]
fn tag_2_with_a_ptadpcm_block_decodes_like_tag_8311() -> TestResult {
    let samples = test_signal(1000, 2);
    let proper = TestWem::ptadpcm(2, 48_000, 260, &samples).bytes();
    let mut relabelled = proper.clone();
    if let Some(tag) = relabelled.get_mut(FMT_BODY..FMT_BODY + 2) {
        tag.copy_from_slice(&2u16.to_le_bytes());
    }

    let wem = Wem::parse(relabelled.as_slice())?;
    let mut wav = Vec::new();
    wem.write_wav(&mut wav)?;
    let mut expected = Vec::new();
    Wem::parse(proper.as_slice())?.write_wav(&mut expected)?;

    assert_eq!(wem.codec(), Codec::PtAdpcm);
    assert_eq!(wem.format_tag(), 2);
    assert_eq!(wav, expected);
    Ok(())
}
