//! `ChunkFile::chunk_data_to_end` reaches data behind a chunk's computed span, which `.dba`
//! animation databases need.

use p4kx_chunks::{ChunkError, ChunkFile, ivo};
use p4kx_testkit::{TestAnimation, TestDba, TestTrack};

fn database() -> Vec<u8> {
    TestDba::new()
        .animation(TestAnimation::new("a.caf", 30).track(
            TestTrack::new(1).rotations(&[(0, [0.0, 0.0, 0.0, 1.0]), (9, [0.0, 0.0, 1.0, 0.0])]),
        ))
        .pool_after_metadata()
        .bytes()
}

#[test]
fn chunk_data_stops_at_the_span_and_the_accessor_reaches_the_pool() -> Result<(), ChunkError> {
    let bytes = database();
    let file = ChunkFile::parse(bytes.as_slice())?;
    let (index, chunk) = file
        .chunks_of(ivo::DBA_DATA)
        .next()
        .expect("the database has a DbaData chunk");

    let bounded = file.chunk_data(index).map(<[u8]>::len);
    let unbounded = file.chunk_data_to_end(index);

    assert_eq!(
        bounded,
        usize::try_from(chunk.size()).ok(),
        "chunk_data is the computed span"
    );
    let unbounded = unbounded.unwrap_or_default();
    assert_eq!(
        unbounded.len(),
        bytes.len() - usize::try_from(chunk.offset()).unwrap_or(0),
        "the accessor runs to the end of the file"
    );
    assert!(
        bounded.is_some_and(|span| unbounded.len() > span),
        "the accessor reaches past the span"
    );
    let pool = file.chunk_data(file.chunks().len() - 1).unwrap_or_default();
    assert!(!pool.is_empty(), "the pool chunk holds the keys");
    assert!(unbounded.ends_with(pool), "the pool is within reach");
    Ok(())
}

#[test]
fn no_chunk_gives_no_data() -> Result<(), ChunkError> {
    let bytes = database();
    let file = ChunkFile::parse(bytes.as_slice())?;

    assert_eq!(
        file.chunk_data_to_end(file.chunks().len()),
        None,
        "no such chunk"
    );
    Ok(())
}
