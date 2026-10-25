// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{HdrImage, RgbaImage, TextureError};

const FACES: usize = 6;
const RGBA: usize = 4;
const RGB: usize = 3;
const CROSS_COLUMNS: u32 = 4;
const CROSS_ROWS: u32 = 3;
const CELLS: [(usize, usize); FACES] = [(2, 1), (0, 1), (1, 0), (1, 2), (1, 1), (3, 1)];

/// Lays the six faces of a cubemap out as a horizontal cross four faces wide and three high.
///
/// `faces` come in DDS order +X, −X, +Y, −Y, +Z, −Z; +Y sits above +Z, the middle row is −X, +Z,
/// +X, −Z and −Y sits below +Z. The cells without a face are transparent black.
///
/// # Errors
///
/// Returns [`TextureError::NotACube`] when `faces` are not six images of one size, and
/// [`TextureError::InvalidHeader`] when the faces are empty or the cross's size overflows.
pub fn cube_cross(faces: &[RgbaImage]) -> Result<RgbaImage, TextureError> {
    let faces: Vec<Face<'_, u8>> = faces
        .iter()
        .map(|face| (face.width(), face.height(), face.pixels()))
        .collect();
    let (width, height, pixels) = cross(&faces, RGBA)?;
    RgbaImage::new(width, height, pixels)
}

/// Lays the six faces of a high-range cubemap out as a horizontal cross like [`cube_cross`];
/// the cells without a face are zero. The cross has an alpha channel when every face has one.
///
/// # Errors
///
/// Returns the errors of [`cube_cross`].
pub fn cube_cross_hdr(hdr_faces: &[HdrImage]) -> Result<HdrImage, TextureError> {
    let faces: Vec<Face<'_, f32>> = hdr_faces
        .iter()
        .map(|face| (face.width(), face.height(), face.pixels()))
        .collect();
    let (width, height, pixels) = cross(&faces, RGB)?;
    let image = HdrImage::new(width, height, pixels)?;
    let alphas: Option<Vec<Face<'_, f32>>> = hdr_faces
        .iter()
        .map(|face| Some((face.width(), face.height(), face.alpha()?)))
        .collect();
    match alphas {
        Some(alphas) => {
            let (_, _, alpha) = cross(&alphas, 1)?;
            image.with_alpha(alpha)
        }
        None => Ok(image),
    }
}

/// The width, height and pixels of one face.
type Face<'a, T> = (u32, u32, &'a [T]);

fn cross<T: Copy + Default>(
    faces: &[Face<'_, T>],
    channels: usize,
) -> Result<(u32, u32, Vec<T>), TextureError> {
    let not_a_cube = || TextureError::NotACube { faces: faces.len() };
    let [(width, height, _), ..] = *faces else {
        return Err(not_a_cube());
    };
    if faces.len() != FACES
        || faces
            .iter()
            .any(|&(face_width, face_height, _)| (face_width, face_height) != (width, height))
    {
        return Err(not_a_cube());
    }
    if width == 0 || height == 0 {
        return Err(TextureError::InvalidHeader("width or height is zero"));
    }
    let overflow = || TextureError::InvalidHeader("the texture size overflows");
    let cross_width = width.checked_mul(CROSS_COLUMNS).ok_or_else(overflow)?;
    let cross_height = height.checked_mul(CROSS_ROWS).ok_or_else(overflow)?;
    let to_usize = |value: u32| usize::try_from(value).map_err(|_| overflow());
    let face_row = to_usize(width)?
        .checked_mul(channels)
        .ok_or_else(overflow)?;
    let cross_row = to_usize(cross_width)?
        .checked_mul(channels)
        .ok_or_else(overflow)?;
    let rows = to_usize(height)?;
    let len = cross_row
        .checked_mul(to_usize(cross_height)?)
        .ok_or_else(overflow)?;
    let mut pixels = vec![T::default(); len];
    for (&(_, _, face), (column, row)) in faces.iter().zip(CELLS) {
        let first_row = row.checked_mul(rows).ok_or_else(overflow)?;
        let target_rows = pixels.chunks_exact_mut(cross_row).skip(first_row);
        for (target, source) in target_rows.zip(face.chunks_exact(face_row)) {
            let cell = column
                .checked_mul(face_row)
                .and_then(|start| target.get_mut(start..start.checked_add(face_row)?))
                .ok_or_else(overflow)?;
            cell.copy_from_slice(source);
        }
    }
    Ok((cross_width, cross_height, pixels))
}
