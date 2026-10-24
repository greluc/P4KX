// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

const MAGIC: &[u8; 4] = b"glTF";
const JSON: u32 = 0x4E4F_534A;
const BIN: u32 = 0x004E_4942;
const HEADER_LEN: usize = 12;
const CHUNK_HEADER_LEN: usize = 8;

/// Splits a GLB file into its JSON and binary chunks after checking the container.
pub(crate) fn split_glb(bytes: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .and_then(|word| <[u8; 4]>::try_from(word).ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| format!("the GLB ends at {} bytes", bytes.len()))
    };
    if bytes.get(..4) != Some(MAGIC.as_slice()) {
        return Err("no glTF magic".to_owned());
    }
    if word(4)? != 2 {
        return Err("GLB version is not 2".to_owned());
    }
    if word(8)? as usize != bytes.len() {
        return Err(format!(
            "the header declares {} bytes, the file has {}",
            word(8)?,
            bytes.len()
        ));
    }
    let (json, after_json) = chunk(bytes, HEADER_LEN, JSON, word)?;
    if json
        .last()
        .is_some_and(|&last| last != b'}' && last != b' ')
    {
        return Err("the JSON chunk is not padded with spaces".to_owned());
    }
    if after_json == bytes.len() {
        return Ok((json, &[]));
    }
    let (blob, end) = chunk(bytes, after_json, BIN, word)?;
    if end != bytes.len() {
        return Err("bytes follow the binary chunk".to_owned());
    }
    Ok((json, blob))
}

fn chunk(
    bytes: &[u8],
    at: usize,
    kind: u32,
    word: impl Fn(usize) -> Result<u32, String>,
) -> Result<(&[u8], usize), String> {
    let len = word(at)? as usize;
    if word(at + 4)? != kind {
        return Err(format!("chunk at {at} has the wrong type"));
    }
    if !len.is_multiple_of(4) {
        return Err(format!("chunk at {at} is not padded to 4 bytes"));
    }
    let start = at + CHUNK_HEADER_LEN;
    let data = bytes
        .get(start..start + len)
        .ok_or_else(|| format!("chunk at {at} exceeds the file"))?;
    Ok((data, start + len))
}
