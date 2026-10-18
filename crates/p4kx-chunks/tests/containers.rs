//! Opening `#ivo` and `CrCh` chunk files and reading their chunk tables and data.

use p4kx_chunks::{ByteOrder, Chunk, ChunkError, ChunkFile, ChunkType, Container, KnownChunkType};
use p4kx_testkit::TestChunkFile;

const IVO_MESH: u32 = 0x9293_B9D8;
const IVO_MTL_NAME: u32 = 0x8335_674E;
const CRCH_NODE: u32 = 0x100B;
const CRCH_DATA_STREAM: u32 = 0x1016;

#[test]
fn an_ivo_file_lists_its_chunks_with_type_version_offset_and_size() -> Result<(), ChunkError> {
    let fixture = TestChunkFile::ivo()
        .chunk(IVO_MTL_NAME, 0x900, *b"material")
        .chunk(IVO_MESH, 0x901, [7; 24]);
    let bytes = fixture.bytes();
    let offsets = fixture.offsets();

    let file = ChunkFile::parse(bytes.as_slice())?;

    assert_eq!(file.container(), Container::Ivo, "container");
    let chunks = file.chunks();
    assert_eq!(chunks.len(), 2, "chunk count");
    let [material, mesh] = chunks else {
        unreachable!("two chunks were asserted");
    };
    assert_eq!(material.chunk_type().name(), Some("MtlName"), "first type");
    assert_eq!(material.version(), 0x900, "first version");
    assert_eq!(
        material.offset(),
        offsets.first().copied().unwrap_or_default(),
        "first offset"
    );
    assert_eq!(material.size(), 8, "first size up to the next chunk");
    assert_eq!(material.id(), None, "#ivo chunks have no ID");
    assert_eq!(
        mesh.chunk_type(),
        ChunkType::new(Container::Ivo, IVO_MESH),
        "second type"
    );
    assert_eq!(mesh.chunk_type().name(), Some("Mesh"), "second name");
    assert_eq!(mesh.version(), 0x901, "second version");
    assert_eq!(mesh.size(), 24, "last size up to the end of the file");
    assert_eq!(
        mesh.byte_order(),
        ByteOrder::Little,
        "#ivo is little-endian"
    );
    assert_eq!(
        file.chunk_data(0),
        Some(b"material".as_slice()),
        "first data"
    );
    assert_eq!(file.chunk_data(1), Some([7; 24].as_slice()), "second data");
    assert_eq!(file.chunk_data(2), None, "no third chunk");
    Ok(())
}

#[test]
fn an_ivo_chunk_size_includes_the_alignment_padding() -> Result<(), ChunkError> {
    let bytes = TestChunkFile::ivo()
        .chunk(IVO_MTL_NAME, 0x900, *b"abc")
        .chunk(IVO_MESH, 0x900, [1; 8])
        .bytes();

    let file = ChunkFile::parse(bytes)?;

    let sizes: Vec<u64> = file.chunks().iter().map(Chunk::size).collect();
    assert_eq!(sizes, [8, 8], "sizes reach the next aligned chunk");
    assert_eq!(
        file.chunk_data(0),
        Some(b"abc\0\0\0\0\0".as_slice()),
        "padded data"
    );
    Ok(())
}

#[test]
fn a_crch_file_lists_its_chunks_with_ids_and_exact_sizes() -> Result<(), ChunkError> {
    let bytes = TestChunkFile::crch()
        .chunk(CRCH_NODE, 0x824, *b"node")
        .chunk(CRCH_DATA_STREAM, 0x800, *b"stream data")
        .bytes();

    let file = ChunkFile::parse(bytes)?;

    assert_eq!(file.container(), Container::CrCh, "container");
    let summary: Vec<_> = file
        .chunks()
        .iter()
        .map(|chunk| {
            (
                chunk.chunk_type().name(),
                chunk.version(),
                chunk.id(),
                chunk.size(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (Some("Node"), 0x824, Some(1), 4),
            (Some("DataStream"), 0x800, Some(2), 11)
        ],
        "chunk table"
    );
    assert_eq!(file.chunk_data(1), Some(b"stream data".as_slice()), "data");
    Ok(())
}

#[test]
fn a_crch_chunk_flagged_big_endian_reports_its_byte_order() -> Result<(), ChunkError> {
    let bytes = TestChunkFile::crch()
        .chunk(CRCH_NODE, 0x824, [1, 2])
        .big_endian_chunk(CRCH_DATA_STREAM, 0x800, [0, 0, 0, 5])
        .bytes();

    let file = ChunkFile::parse(bytes)?;

    let orders: Vec<_> = file
        .chunks()
        .iter()
        .map(|chunk| (chunk.byte_order(), chunk.version()))
        .collect();
    assert_eq!(
        orders,
        [(ByteOrder::Little, 0x824), (ByteOrder::Big, 0x800)],
        "byte order per chunk, version without the flag"
    );
    Ok(())
}

#[test]
fn unknown_chunk_types_are_listed_and_flagged() -> Result<(), ChunkError> {
    let bytes = TestChunkFile::ivo()
        .chunk(0x1234_5678, 1, [0; 8])
        .chunk(IVO_MESH, 0x900, [0; 8])
        .bytes();

    let file = ChunkFile::parse(bytes)?;

    let unknown = file.chunks().first().map(Chunk::chunk_type);
    assert_eq!(
        unknown.map(ChunkType::is_known),
        Some(false),
        "flagged unknown"
    );
    assert_eq!(unknown.and_then(ChunkType::name), None, "no name");
    assert_eq!(
        unknown.map(|chunk_type| chunk_type.to_string()),
        Some("unknown 0x12345678".to_owned()),
        "displayed with its ID"
    );
    assert_eq!(
        file.chunk_data(0),
        Some([0; 8].as_slice()),
        "data still readable"
    );
    Ok(())
}

#[test]
fn an_empty_chunk_table_is_valid() -> Result<(), ChunkError> {
    let file = ChunkFile::parse(TestChunkFile::crch().bytes())?;

    assert!(file.chunks().is_empty(), "no chunks");
    Ok(())
}

#[test]
fn every_known_type_is_found_by_its_id() {
    for container in [Container::Ivo, Container::CrCh] {
        for known in KnownChunkType::table(container) {
            let chunk_type = ChunkType::new(container, known.id);
            assert_eq!(chunk_type.name(), Some(known.name), "{container} {known:?}");
            assert_eq!(chunk_type.to_string(), known.name, "{container} display");
        }
    }
}

#[test]
fn the_same_id_is_named_per_container() {
    assert_eq!(
        ChunkType::new(Container::CrCh, 0x1000).name(),
        Some("Mesh"),
        "CrCh"
    );
    assert_eq!(ChunkType::new(Container::Ivo, 0x1000).name(), None, "#ivo");
    assert_eq!(
        ChunkType::new(Container::CrCh, 0x1000).hex_id(),
        "0x1000",
        "16-bit ID"
    );
    assert_eq!(
        ChunkType::new(Container::Ivo, 0x1000).hex_id(),
        "0x00001000",
        "32-bit ID"
    );
}
