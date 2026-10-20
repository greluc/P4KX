use p4kx_foundation::ByteReader;

use crate::CorruptChunk;
use crate::reader::to_usize;
use crate::vector::{
    dequantise, half, normal_of_quaternion, normal_of_tangent_frame, normalise, unpack_unit_vector,
};

const INDICES: u32 = 0xEECD_C168;
const VERTS_UVS: u32 = 0x9132_9AE9;
const VERTS_UVS_2: u32 = 0xB3A7_0D5E;
const NORMALS: u32 = 0x9CF3_F615;
const NORMALS_2: u32 = 0x38A5_81FE;
const TANGENTS: u32 = 0xB95E_9A1B;
const QTANGENTS: u32 = 0xEE05_7252;
const STREAM_ALIGNMENT: usize = 8;

/// The vertex positions and texture coordinates of a mesh.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VertexStream<'a> {
    data: &'a [u8],
    layout: VertexLayout,
}

#[derive(Debug, Clone, Copy)]
enum VertexLayout {
    Quantised { min: [f32; 3], max: [f32; 3] },
    Float,
}

impl VertexStream<'_> {
    const fn stride(&self) -> usize {
        match self.layout {
            VertexLayout::Quantised { .. } => 16,
            VertexLayout::Float => 20,
        }
    }

    /// Returns the position and texture coordinate of vertex `index`.
    pub(crate) fn vertex(&self, index: usize) -> Result<([f32; 3], [f32; 2]), CorruptChunk> {
        let stride = self.stride();
        let bytes = element(self.data, index, stride, "vertex")?;
        let word = |at: usize| u32_at(bytes, at);
        let position = match self.layout {
            VertexLayout::Quantised { min, max } => {
                let raw = [0, 2, 4].map(|at| i16_at(bytes, at));
                dequantise(raw, min, max)
            }
            VertexLayout::Float => [0, 4, 8].map(|at| f32::from_bits(word(at))),
        };
        let uv_at = stride - 4;
        let uv = [uv_at, uv_at + 2].map(|at| half(u16_at(bytes, at)));
        if position.iter().any(|value| !value.is_finite()) {
            return Err(CorruptChunk::NonFinite("vertex position"));
        }
        if uv.iter().any(|value| !value.is_finite()) {
            return Err(CorruptChunk::NonFinite("texture coordinate"));
        }
        Ok((position, uv))
    }
}

/// The vertex normals of a mesh in one of their stored forms.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NormalStream<'a> {
    data: &'a [u8],
    kind: NormalKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormalKind {
    Float,
    Packed,
    TangentFrame,
    Quaternion,
}

impl NormalKind {
    const fn stride(self) -> usize {
        match self {
            Self::Float => 12,
            Self::Packed => 4,
            Self::TangentFrame | Self::Quaternion => 8,
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Float => 3,
            Self::Packed => 2,
            Self::TangentFrame | Self::Quaternion => 1,
        }
    }
}

impl NormalStream<'_> {
    /// Returns the unit normal of vertex `index`.
    pub(crate) fn normal(&self, index: usize) -> Result<[f32; 3], CorruptChunk> {
        let bytes = element(self.data, index, self.kind.stride(), "normal")?;
        let word = |at: usize| u32_at(bytes, at);
        let normal = match self.kind {
            NormalKind::Float => normalise([0, 4, 8].map(|at| f32::from_bits(word(at)))),
            NormalKind::Packed => unpack_unit_vector(word(0)),
            NormalKind::TangentFrame => normal_of_tangent_frame(word(0), word(4)),
            NormalKind::Quaternion => {
                normal_of_quaternion([0, 2, 4, 6].map(|at| i16_at(bytes, at)))
            }
        };
        if normal.iter().any(|value| !value.is_finite()) {
            return Err(CorruptChunk::NonFinite("normal"));
        }
        Ok(normal)
    }
}

/// The index list of a mesh, 16 or 32 bits per index.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IndexStream<'a> {
    data: &'a [u8],
    wide: bool,
}

impl IndexStream<'_> {
    /// Returns index `position` of the list.
    pub(crate) fn index(&self, position: usize) -> Result<u32, CorruptChunk> {
        if self.wide {
            element(self.data, position, 4, "index").map(|bytes| u32_at(bytes, 0))
        } else {
            element(self.data, position, 2, "index").map(|bytes| u32::from(u16_at(bytes, 0)))
        }
    }
}

/// The data streams of a mesh chunk that a model reads.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Streams<'a> {
    pub(crate) vertices: VertexStream<'a>,
    pub(crate) normals: Option<NormalStream<'a>>,
    pub(crate) indices: IndexStream<'a>,
}

/// The counts and bounding box the streams are read with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StreamCounts {
    pub(crate) vertices: u32,
    pub(crate) indices: u32,
    pub(crate) min: [f32; 3],
    pub(crate) max: [f32; 3],
}

/// Reads the streams that follow the submeshes until the end of the chunk; each is a type tag,
/// an element size and one element per vertex or index, padded to 8 bytes.
pub(crate) fn read_streams<'a>(
    reader: &mut ByteReader<'a>,
    counts: StreamCounts,
) -> Result<Streams<'a>, CorruptChunk> {
    let mut vertices = None;
    let mut normals: Option<NormalStream<'a>> = None;
    let mut indices = None;
    let mut seen = 0_u8;
    while reader.remaining() >= 4 {
        let tag = reader.read_u32()?;
        if tag == 0 {
            continue;
        }
        let size = reader.read_u32()?;
        if let Some(bit) = read_tag_bit(tag) {
            if seen & bit != 0 {
                return Err(CorruptChunk::RepeatedStream(tag));
            }
            seen |= bit;
        }
        let count = if tag == INDICES {
            counts.indices
        } else {
            counts.vertices
        };
        let len = to_usize(size).checked_mul(to_usize(count));
        let known =
            matches!(tag, INDICES | VERTS_UVS | VERTS_UVS_2) || normal_kind(tag, size).is_some();
        let Some(len) = len.filter(|&len| known || len <= reader.remaining()) else {
            break;
        };
        let data = reader.read_bytes(len)?;
        let padding = len.next_multiple_of(STREAM_ALIGNMENT) - len;
        reader.skip(padding.min(reader.remaining()))?;
        match classify(tag, size, data, counts)? {
            Some(Found::Indices(stream)) => indices = Some(stream),
            Some(Found::Vertices(stream)) => vertices = Some(stream),
            Some(Found::Normals(stream))
                if normals.is_none_or(|known| known.kind.rank() < stream.kind.rank()) =>
            {
                normals = Some(stream);
            }
            Some(Found::Normals(_)) | None => {}
        }
    }
    Ok(Streams {
        vertices: vertices.ok_or(CorruptChunk::MissingStream("vertex"))?,
        normals,
        indices: indices.ok_or(CorruptChunk::MissingStream("index"))?,
    })
}

const fn read_tag_bit(tag: u32) -> Option<u8> {
    match tag {
        INDICES => Some(1),
        VERTS_UVS => Some(1 << 1),
        VERTS_UVS_2 => Some(1 << 2),
        NORMALS => Some(1 << 3),
        NORMALS_2 => Some(1 << 4),
        TANGENTS => Some(1 << 5),
        QTANGENTS => Some(1 << 6),
        _ => None,
    }
}

enum Found<'a> {
    Indices(IndexStream<'a>),
    Vertices(VertexStream<'a>),
    Normals(NormalStream<'a>),
}

fn classify(
    tag: u32,
    size: u32,
    data: &[u8],
    counts: StreamCounts,
) -> Result<Option<Found<'_>>, CorruptChunk> {
    let (min, max) = (counts.min, counts.max);
    Ok(match (tag, size) {
        (INDICES, 2 | 4) => Some(Found::Indices(IndexStream {
            data,
            wide: size == 4,
        })),
        (VERTS_UVS | VERTS_UVS_2, 16) => Some(Found::Vertices(VertexStream {
            data,
            layout: VertexLayout::Quantised { min, max },
        })),
        (VERTS_UVS | VERTS_UVS_2, 20) => Some(Found::Vertices(VertexStream {
            data,
            layout: VertexLayout::Float,
        })),
        (INDICES | VERTS_UVS | VERTS_UVS_2, _) => {
            return Err(CorruptChunk::ElementSize {
                stream: name(tag),
                size,
            });
        }
        _ => normal_kind(tag, size).map(|kind| Found::Normals(NormalStream { data, kind })),
    })
}
const fn normal_kind(tag: u32, size: u32) -> Option<NormalKind> {
    match (tag, size) {
        (NORMALS_2, 12) => Some(NormalKind::Float),
        (NORMALS, 4) => Some(NormalKind::Packed),
        (QTANGENTS, 8) => Some(NormalKind::TangentFrame),
        (TANGENTS, 8) => Some(NormalKind::Quaternion),
        _ => None,
    }
}

const fn name(tag: u32) -> &'static str {
    match tag {
        INDICES => "index",
        VERTS_UVS | VERTS_UVS_2 => "vertex",
        _ => "data",
    }
}

fn element<'a>(
    data: &'a [u8],
    index: usize,
    stride: usize,
    stream: &'static str,
) -> Result<&'a [u8], CorruptChunk> {
    index
        .checked_mul(stride)
        .and_then(|start| data.get(start..start.checked_add(stride)?))
        .ok_or(CorruptChunk::MissingStream(stream))
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    let pair = bytes
        .get(at..at + 2)
        .and_then(|pair| <[u8; 2]>::try_from(pair).ok());
    pair.map_or(0, u16::from_le_bytes)
}

fn i16_at(bytes: &[u8], at: usize) -> i16 {
    i16::from_le_bytes(u16_at(bytes, at).to_le_bytes())
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    let word = bytes
        .get(at..at + 4)
        .and_then(|word| <[u8; 4]>::try_from(word).ok());
    word.map_or(0, u32::from_le_bytes)
}
