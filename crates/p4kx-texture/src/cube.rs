use crate::{RgbaImage, TextureError};

const FACES: usize = 6;
const RGBA: usize = 4;
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
    let not_a_cube = || TextureError::NotACube { faces: faces.len() };
    let [first, ..] = faces else {
        return Err(not_a_cube());
    };
    let (width, height) = (first.width(), first.height());
    if faces.len() != FACES
        || faces
            .iter()
            .any(|face| (face.width(), face.height()) != (width, height))
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
    let face_row = to_usize(width)?.checked_mul(RGBA).ok_or_else(overflow)?;
    let cross_row = to_usize(cross_width)?
        .checked_mul(RGBA)
        .ok_or_else(overflow)?;
    let rows = to_usize(height)?;
    let len = cross_row
        .checked_mul(to_usize(cross_height)?)
        .ok_or_else(overflow)?;
    let mut pixels = vec![0; len];
    for (face, (column, row)) in faces.iter().zip(CELLS) {
        let first_row = row.checked_mul(rows).ok_or_else(overflow)?;
        let target_rows = pixels.chunks_exact_mut(cross_row).skip(first_row);
        for (target, source) in target_rows.zip(face.pixels().chunks_exact(face_row)) {
            let cell = column
                .checked_mul(face_row)
                .and_then(|start| target.get_mut(start..start.checked_add(face_row)?))
                .ok_or_else(overflow)?;
            cell.copy_from_slice(source);
        }
    }
    RgbaImage::new(cross_width, cross_height, pixels)
}
