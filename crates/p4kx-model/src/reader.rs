use p4kx_foundation::{ByteReader, ParseError};

/// Reads the little-endian values model chunks are made of.
pub(crate) trait ReadValues {
    /// Reads a little-endian `f32`.
    fn read_f32(&mut self) -> Result<f32, ParseError>;

    /// Reads three little-endian `f32`s.
    fn read_vec3(&mut self) -> Result<[f32; 3], ParseError>;

    /// Reads a 3×4 row-major matrix of little-endian `f32`s.
    fn read_rows(&mut self) -> Result<[[f32; 4]; 3], ParseError>;
}

impl ReadValues for ByteReader<'_> {
    fn read_f32(&mut self) -> Result<f32, ParseError> {
        self.read_u32().map(f32::from_bits)
    }

    fn read_vec3(&mut self) -> Result<[f32; 3], ParseError> {
        Ok([self.read_f32()?, self.read_f32()?, self.read_f32()?])
    }

    fn read_rows(&mut self) -> Result<[[f32; 4]; 3], ParseError> {
        let mut rows = [[0.0; 4]; 3];
        for value in rows.iter_mut().flatten() {
            *value = self.read_f32()?;
        }
        Ok(rows)
    }
}

/// Converts a count read from a file to `usize`, saturating where `usize` is narrower.
pub(crate) fn to_usize(count: u32) -> usize {
    usize::try_from(count).unwrap_or(usize::MAX)
}
