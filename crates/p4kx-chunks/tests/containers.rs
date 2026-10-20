//! Opening `#ivo` and `CrCh` chunk files and reading their chunk tables and data.

use p4kx_chunks::{
    ByteOrder, Chunk, ChunkError, ChunkFile, ChunkType, Container, KnownChunkType, crch, ivo,
};
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
fn the_tables_ascend_by_id_without_repeats() {
    for container in [Container::Ivo, Container::CrCh] {
        let ids: Vec<u32> = KnownChunkType::table(container)
            .iter()
            .map(|known| known.id)
            .collect();
        assert!(
            ids.windows(2).all(|pair| pair[0] < pair[1]),
            "{container} table is strictly ascending"
        );
    }
}

#[test]
fn the_ivo_ids_measured_in_real_archives_have_their_names() {
    let named = [
        (0x2B7E_CF9F, "PositionBonemap"),
        (0x58DE_1772, "StatObjPhysics"),
        (0xBE5E_493E, "ExportFlags"),
        (0x9291_4444, "MeshInfo"),
        (0x57A3_8888, "BShapesGpu"),
        (0xF5C6_EB5B, "BShapes"),
        (0x0A24_85B6, "RigLogic"),
        (0x1BBC_4103, "Skeleton"),
        (0x7E03_5555, "RigInfo"),
        (0xF708_6666, "ProtosInfo"),
        (0x90C6_2222, "PhysicalHierarchy"),
        (0xB324_59D2, "VisAreas"),
        (0xB875_7777, "Skin2"),
    ];
    for (id, name) in named {
        assert_eq!(
            ChunkType::new(Container::Ivo, id).name(),
            Some(name),
            "{id:#X}"
        );
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

#[test]
fn an_ivo_chunk_before_a_trailing_chunk_table_ends_at_the_table() -> Result<(), ChunkError> {
    let bytes = TestChunkFile::ivo()
        .chunk(IVO_MTL_NAME, 0x900, *b"material")
        .chunk(IVO_MESH, 0x900, [3; 16])
        .table_at_end()
        .bytes();

    let file = ChunkFile::parse(bytes.as_slice())?;

    let sizes: Vec<u64> = file.chunks().iter().map(Chunk::size).collect();
    assert_eq!(sizes, [8, 16], "the last chunk stops at the table");
    assert_eq!(
        file.chunk_data(1),
        Some([3; 16].as_slice()),
        "data without the table"
    );
    Ok(())
}

fn swap_ivo_entries(bytes: &mut [u8], first: usize, second: usize) {
    let entry = |index: usize| 16 + index * 16;
    let (low, high) = (entry(first.min(second)), entry(first.max(second)));
    if let Some((head, tail)) = bytes.split_at_mut_checked(high)
        && let (Some(a), Some(b)) = (head.get_mut(low..low + 16), tail.get_mut(..16))
    {
        a.swap_with_slice(b);
    }
}

fn set_ivo_offset(bytes: &mut [u8], index: usize, offset: u64) {
    let at = 16 + index * 16 + 8;
    if let Some(slot) = bytes.get_mut(at..at + 8) {
        slot.copy_from_slice(&offset.to_le_bytes());
    }
}

#[test]
fn an_unsorted_ivo_table_keeps_file_order_and_sizes_by_offset() -> Result<(), ChunkError> {
    let mut bytes = TestChunkFile::ivo()
        .chunk(IVO_MTL_NAME, 0x900, *b"material")
        .chunk(IVO_MESH, 0x900, [4; 24])
        .bytes();
    swap_ivo_entries(&mut bytes, 0, 1);

    let file = ChunkFile::parse(bytes.as_slice())?;

    let summary: Vec<_> = file
        .chunks()
        .iter()
        .map(|chunk| (chunk.chunk_type(), chunk.size()))
        .collect();
    assert_eq!(
        summary,
        [(ivo::MESH, 24), (ivo::MTL_NAME, 8)],
        "table order kept, sizes from the offsets"
    );
    assert_eq!(file.chunk_data(0), Some([4; 24].as_slice()), "mesh data");
    Ok(())
}

#[test]
fn ivo_chunks_that_share_an_offset_share_their_span() -> Result<(), ChunkError> {
    let fixture = TestChunkFile::ivo()
        .chunk(IVO_MTL_NAME, 0x900, *b"material")
        .chunk(IVO_MESH, 0x900, [5; 16])
        .chunk(0xE018_1074, 0x900, [6; 8]);
    let mut bytes = fixture.bytes();
    let first = fixture.offsets().first().copied().unwrap_or_default();
    set_ivo_offset(&mut bytes, 1, first);

    let file = ChunkFile::parse(bytes.as_slice())?;

    let sizes: Vec<u64> = file.chunks().iter().map(Chunk::size).collect();
    assert_eq!(sizes, [24, 24, 8], "both reach the next distinct offset");
    assert_eq!(file.chunk_data(0), file.chunk_data(1), "same bytes");
    Ok(())
}

#[test]
fn chunks_are_found_by_type_and_crch_chunks_by_id() -> Result<(), ChunkError> {
    let bytes = TestChunkFile::crch()
        .chunk(CRCH_NODE, 0x824, *b"root")
        .chunk(CRCH_DATA_STREAM, 0x800, *b"positions")
        .chunk(CRCH_NODE, 0x824, *b"child")
        .bytes();

    let file = ChunkFile::parse(bytes)?;

    let nodes: Vec<usize> = file.chunks_of(crch::NODE).map(|(index, _)| index).collect();
    assert_eq!(nodes, [0, 2], "both nodes in file order");
    assert_eq!(file.chunks_of(crch::MESH).count(), 0, "no mesh");
    let found = file
        .chunk_by_id(2)
        .map(|(index, chunk)| (index, chunk.chunk_type()));
    assert_eq!(found, Some((1, crch::DATA_STREAM)), "chunk ID 2");
    assert_eq!(file.chunk_by_id(9), None, "no chunk ID 9");
    Ok(())
}

#[test]
fn the_named_constants_match_the_table() {
    assert_eq!(ivo::MESH.name(), Some("Mesh"), "#ivo mesh");
    assert_eq!(ivo::MESH.id(), IVO_MESH, "#ivo mesh ID");
    assert_eq!(
        crch::NODE,
        ChunkType::new(Container::CrCh, CRCH_NODE),
        "CrCh node"
    );
    assert_eq!(crch::NODE.name(), Some("Node"), "CrCh node name");
}
