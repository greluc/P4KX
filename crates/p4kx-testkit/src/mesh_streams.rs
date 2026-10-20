use std::ops::Range;

const INDICES: u32 = 0xEECD_C168;
const HEADER_LEN: usize = 168;
const SUBMESH_LEN: usize = 48;
const STREAM_ALIGNMENT: usize = 8;

/// One data stream of a mesh chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshStream {
    /// The stream's type tag.
    pub tag: u32,
    /// The size of one element in bytes.
    pub element_size: u32,
    /// The stream's data within the chunk.
    pub data: Range<usize>,
}

/// The counts and streams of a `Skin` or `Skin2` mesh chunk, walked the way `p4kx-model` reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshStreams {
    /// The number of vertices.
    pub vertices: u32,
    /// The number of indices.
    pub indices: u32,
    /// The streams in chunk order.
    pub streams: Vec<MeshStream>,
    /// Whether a stream reaches past the end of the chunk, which ends the walk.
    pub truncated: bool,
}

fn word(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(bytes).ok()?))
}

/// Walks the streams of a mesh chunk the way the model reader does.
///
/// After the header and the submeshes, each stream is a tag, an element size and one element per
/// vertex (per index for the index stream), followed by padding that rounds the stream's length up
/// to 8 bytes. Returns `None` when the header is cut short.
#[must_use]
pub fn mesh_streams(chunk: &[u8]) -> Option<MeshStreams> {
    let vertices = word(chunk, 8)?;
    let indices = word(chunk, 12)?;
    let submeshes = usize::try_from(word(chunk, 16)?).ok()?;
    let extra = usize::try_from(word(chunk, 76)?).ok()?;
    let mut at = HEADER_LEN
        .checked_add(submeshes.checked_mul(SUBMESH_LEN)?)?
        .checked_add(extra.checked_mul(4)?)?;
    let mut streams = Vec::new();
    let mut truncated = false;
    while at.checked_add(4)? <= chunk.len() {
        let tag = word(chunk, at)?;
        at += 4;
        if tag == 0 {
            continue;
        }
        let element_size = word(chunk, at)?;
        at += 4;
        let count = if tag == INDICES { indices } else { vertices };
        let len = usize::try_from(element_size)
            .ok()?
            .checked_mul(usize::try_from(count).ok()?)?;
        let end = at.checked_add(len).filter(|&end| end <= chunk.len());
        let Some(end) = end else {
            truncated = true;
            break;
        };
        streams.push(MeshStream {
            tag,
            element_size,
            data: at..end,
        });
        let padding = len.next_multiple_of(STREAM_ALIGNMENT) - len;
        at = end + padding.min(chunk.len() - end);
    }
    Some(MeshStreams {
        vertices,
        indices,
        streams,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::mesh_streams;
    use crate::TestModel;

    #[test]
    fn the_streams_of_the_cube_are_walked_with_their_tags_and_sizes() {
        let chunk = TestModel::cube().skin_chunk();

        let walked = mesh_streams(&chunk).expect("a whole header");

        let found: Vec<(u32, u32)> = walked
            .streams
            .iter()
            .map(|stream| (stream.tag, stream.element_size))
            .collect();
        assert_eq!(
            found,
            [
                (0xEECD_C168, 2),
                (0xB3A7_0D5E, 16),
                (0x38A5_81FE, 12),
                (0x9D51_C5EE, 2)
            ],
            "the streams of the fixture"
        );
        assert_eq!((walked.vertices, walked.indices), (24, 36), "counts");
        assert!(!walked.truncated, "whole chunk");
    }

    #[test]
    fn a_stream_past_the_end_stops_the_walk() {
        let mut chunk = TestModel::cube().skin_chunk();
        chunk.truncate(chunk.len() - 10);

        let walked = mesh_streams(&chunk).expect("a whole header");

        assert!(walked.truncated, "cut stream");
    }
}
