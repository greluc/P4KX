// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Bounds-checked little-endian reads at an offset, for the offset-addressed layout of DNA rigs.

fn array<const N: usize>(data: &[u8], at: usize) -> Option<[u8; N]> {
    let end = at.checked_add(N)?;
    data.get(at..end)?.try_into().ok()
}

pub(crate) fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    array(data, at).map(u16::from_le_bytes)
}

pub(crate) fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    array(data, at).map(u32::from_le_bytes)
}

pub(crate) fn u64_at(data: &[u8], at: usize) -> Option<u64> {
    array(data, at).map(u64::from_le_bytes)
}

pub(crate) fn f32_at(data: &[u8], at: usize) -> Option<f32> {
    array(data, at).map(f32::from_le_bytes)
}

pub(crate) fn f64_at(data: &[u8], at: usize) -> Option<f64> {
    array(data, at).map(f64::from_le_bytes)
}

pub(crate) fn vec3_at(data: &[u8], at: usize) -> Option<[f32; 3]> {
    Some([
        f32_at(data, at)?,
        f32_at(data, at.checked_add(4)?)?,
        f32_at(data, at.checked_add(8)?)?,
    ])
}
