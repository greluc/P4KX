// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! An odd-sized chunk may or may not be followed by a pad byte; the next chunk is found either way.

use p4kx_audio::{AudioError, Codec, Wem};

fn chunk(out: &mut Vec<u8>, id: [u8; 4], body: &[u8], pad: bool) {
    out.extend_from_slice(&id);
    out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(body);
    if pad && body.len() % 2 == 1 {
        out.push(0);
    }
}

fn wem(odd: &[u8], data: &[u8], pad: bool) -> Vec<u8> {
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&1u16.to_le_bytes());
    fmt.extend_from_slice(&1u16.to_le_bytes());
    fmt.extend_from_slice(&48_000u32.to_le_bytes());
    fmt.extend_from_slice(&96_000u32.to_le_bytes());
    fmt.extend_from_slice(&2u16.to_le_bytes());
    fmt.extend_from_slice(&16u16.to_le_bytes());
    let mut body = b"WAVE".to_vec();
    chunk(&mut body, *b"fmt ", &fmt, pad);
    chunk(&mut body, *b"akd ", odd, pad);
    chunk(&mut body, *b"data", data, pad);
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

const DATA_LENS: [usize; 8] = [4, 0x20, 0x40, 0x42, 0x7E, 0x120, 0x2000, 0x4141];

fn data_of(len: usize) -> Vec<u8> {
    (0..len)
        .map(|byte| u8::try_from(byte % 7).unwrap_or(0))
        .collect()
}

#[test]
fn a_padded_odd_chunk_is_followed_by_the_next_chunk() -> Result<(), AudioError> {
    for len in DATA_LENS {
        let data = data_of(len);
        let bytes = wem(&[7; 5], &data, true);

        assert_eq!(
            Wem::parse(bytes.as_slice())?.data(),
            data,
            "{len} data bytes"
        );
    }
    Ok(())
}

#[test]
fn an_unpadded_odd_chunk_as_wwise_writes_it_is_followed_by_the_next_chunk() -> Result<(), AudioError>
{
    for len in DATA_LENS {
        let data = data_of(len);
        let bytes = wem(&[7; 5], &data, false);

        let wem = Wem::parse(bytes.as_slice())?;

        assert_eq!(wem.data(), data, "{len} data bytes");
        assert_eq!(wem.codec(), Codec::Pcm);
    }
    Ok(())
}

#[test]
fn an_odd_last_chunk_and_a_truncated_file_never_panic() {
    for pad in [true, false] {
        let bytes = wem(&[7; 5], &[0; 32], pad);
        for len in 0..bytes.len() {
            let _ = Wem::parse(bytes.get(..len).unwrap_or_default());
        }
    }
}
