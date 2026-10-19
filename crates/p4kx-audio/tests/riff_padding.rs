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

fn wem(odd: &[u8], pad: bool) -> Vec<u8> {
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
    chunk(&mut body, *b"data", &[1, 0, 2, 0], pad);
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[test]
fn a_padded_odd_chunk_is_followed_by_the_next_chunk() -> Result<(), AudioError> {
    let bytes = wem(&[7; 5], true);

    assert_eq!(Wem::parse(bytes.as_slice())?.data(), [1, 0, 2, 0]);
    Ok(())
}

#[test]
fn an_unpadded_odd_chunk_as_wwise_writes_it_is_followed_by_the_next_chunk() -> Result<(), AudioError>
{
    let bytes = wem(&[7; 5], false);

    let wem = Wem::parse(bytes.as_slice())?;

    assert_eq!(wem.data(), [1, 0, 2, 0], "data");
    assert_eq!(wem.codec(), Codec::Pcm);
    Ok(())
}

#[test]
fn an_odd_last_chunk_and_a_truncated_file_never_panic() {
    let bytes = wem(&[7; 5], false);
    for len in 0..bytes.len() {
        let _ = Wem::parse(bytes.get(..len).unwrap_or_default());
    }
}
