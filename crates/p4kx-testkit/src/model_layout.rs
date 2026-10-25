// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// How a [`TestModel`] stores its vertex positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPositions {
    /// 16-byte vertices with positions as signed 16-bit values scaled to the mesh's bounding box.
    Quantized,
    /// 20-byte vertices with positions as three 32-bit floats.
    Float,
}

/// How a [`TestModel`] stores its vertex normals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestNormals {
    /// An `IVONORMALS2` stream of three 32-bit floats per vertex.
    Float,
    /// An `IVOQTANGENTS` stream of a packed tangent and bitangent per vertex.
    Compressed,
    /// An `IVONORMALS` stream of three 32-bit floats per vertex.
    FloatOnNormals,
    /// An `IVONORMALS` stream of one packed unit vector per vertex.
    PackedOnNormals,
    /// An `IVOTANGENTS` stream of 16 bytes per vertex, a layout no real Archive holds and the
    /// reader skips.
    Tangents16,
    /// No normal stream.
    None,
}
