use crate::{RgbaImage, TextureError};

const FACES: usize = 6;
const RGBA: usize = 4;
const CROSS_COLUMNS: usize = 4;
const CROSS_ROWS: usize = 3;
const CELLS: [(usize, usize); FACES] = [(2, 1), (0, 1), (1, 0), (1, 2), (1, 1), (3, 1)];

/// Lays the six faces of a cubemap out as a horizontal cross four faces wide and three high.
///
/// `faces` come in DDS order +X, −X, +Y, −Y, +Z, −Z; +Y sits above +Z, the middle row is −X, +Z,
/// +X, −Z and −Y sits below +Z. The cells without a face are transparent black.
///
/// # Errors
///
/// Returns [`TextureError::NotACube`] when `faces` are not six images of one size, and
/// [`TextureError::InvalidHeader`] when the cross's size overflows.
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
    let overflow = || TextureError::InvalidHeader("the texture size overflows");
    let face_row = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(RGBA))
        .ok_or_else(overflow)?;
    let rows = usize::try_from(height).map_err(|_| overflow())?;
    let cross_row = face_row.checked_mul(CROSS_COLUMNS).ok_or_else(overflow)?;
    let len = cross_row
        .checked_mul(rows)
        .and_then(|len| len.checked_mul(CROSS_ROWS))
        .ok_or_else(overflow)?;
    let mut pixels = vec![0; len];
    for (face, (column, row)) in faces.iter().zip(CELLS) {
        for (y, source) in face.pixels().chunks_exact(face_row).enumerate() {
            let start = (row * rows + y) * cross_row + column * face_row;
            if let Some(target) = pixels.get_mut(start..start + face_row) {
                target.copy_from_slice(source);
            }
        }
    }
    let cross_width = width.checked_mul(4).ok_or_else(overflow)?;
    let cross_height = height.checked_mul(3).ok_or_else(overflow)?;
    RgbaImage::new(cross_width, cross_height, pixels)
}
