//! Survey of the two bounding boxes of the mesh chunks of a real Archive against the extent of the
//! quantised vertices they decode; runs with `--ignored` when `P4KX_DATA_P4K` names one and
//! prints the tables (`--nocapture`).

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const EXTENSIONS: [&str; 4] = ["cgfm", "cgam", "skinm", "chrm"];
const MAX_FILE_LEN: u64 = 512 << 20;
const INDICES: u32 = 0xEECD_C168;
const VERTS_UVS_2: u32 = 0xB3A7_0D5E;
const HEADER_LEN: usize = 24 + 48 + 8 + 88;
const SUBMESH_LEN: usize = 48;
const SNORM16: f32 = 32767.0;

type Axis = (f32, f32);
type Box3 = [Axis; 3];

#[derive(Default)]
struct Tally {
    chunks: u64,
    same: u64,
    same_tight: u64,
    differ: u64,
    first_tight: u64,
    second_tight: u64,
    both_tight: u64,
    neither_tight: u64,
    first_contains: u64,
    second_contains: u64,
    example_differing: String,
}

impl Tally {
    fn merge(&mut self, other: Self) {
        self.chunks += other.chunks;
        self.same += other.same;
        self.same_tight += other.same_tight;
        self.differ += other.differ;
        self.first_tight += other.first_tight;
        self.second_tight += other.second_tight;
        self.both_tight += other.both_tight;
        self.neither_tight += other.neither_tight;
        self.first_contains += other.first_contains;
        self.second_contains += other.second_contains;
        if self.example_differing.is_empty() {
            self.example_differing = other.example_differing;
        }
    }
}

type Survey = BTreeMap<String, Tally>;

fn merge(mut left: Survey, right: Survey) -> Survey {
    for (key, tally) in right {
        left.entry(key).or_default().merge(tally);
    }
    left
}

fn word(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(bytes).ok()?))
}

fn vec3(data: &[u8], at: usize) -> Option<[f32; 3]> {
    let part = |offset: usize| word(data, at + offset).map(f32::from_bits);
    Some([part(0)?, part(4)?, part(8)?])
}

fn box_at(data: &[u8], at: usize) -> Option<Box3> {
    let (min, max) = (vec3(data, at)?, vec3(data, at + 12)?);
    let mut axes = [(0.0, 0.0); 3];
    for ((axis, low), high) in axes.iter_mut().zip(min).zip(max) {
        *axis = (low, high);
    }
    Some(axes)
}

fn quantised_vertices(chunk: &[u8]) -> Option<(usize, usize)> {
    let vertices = usize::try_from(word(chunk, 8)?).ok()?;
    let indices = usize::try_from(word(chunk, 12)?).ok()?;
    let submeshes = usize::try_from(word(chunk, 16)?).ok()?;
    let extra = usize::try_from(word(chunk, 24 + 48 + 4)?).ok()?;
    let mut at = HEADER_LEN
        .checked_add(submeshes.checked_mul(SUBMESH_LEN)?)?
        .checked_add(extra.checked_mul(4)?)?;
    while at + 4 <= chunk.len() {
        let tag = word(chunk, at)?;
        at += 4;
        if tag == 0 {
            continue;
        }
        let size = usize::try_from(word(chunk, at)?).ok()?;
        at += 4;
        let count = if tag == INDICES { indices } else { vertices };
        let end = at.checked_add(size.checked_mul(count)?)?;
        if end > chunk.len() {
            return None;
        }
        if tag == VERTS_UVS_2 && size == 16 {
            return Some((at, vertices));
        }
        at = end.next_multiple_of(8).min(chunk.len());
    }
    None
}

fn raw_extent(chunk: &[u8], start: usize, vertices: usize) -> Option<([i16; 3], [i16; 3])> {
    let mut low = [i16::MAX; 3];
    let mut high = [i16::MIN; 3];
    for vertex in 0..vertices {
        let base = start + vertex * 16;
        let bytes = chunk.get(base..base + 6)?;
        for ((low, high), pair) in low
            .iter_mut()
            .zip(high.iter_mut())
            .zip(bytes.as_chunks::<2>().0)
        {
            let value = i16::from_le_bytes(*pair);
            *low = (*low).min(value);
            *high = (*high).max(value);
        }
    }
    (vertices > 0).then_some((low, high))
}

fn half_of((low, high): Axis) -> f32 {
    ((high - low) / 2.0).max(1.0)
}

fn tolerance(axis: Axis) -> f32 {
    (half_of(axis) / 8192.0).max(1e-3)
}

fn decode(raw: [i16; 3], axes: Box3) -> [f32; 3] {
    let mut out = [0.0; 3];
    for ((value, raw), axis) in out.iter_mut().zip(raw).zip(axes) {
        *value = (f32::from(raw) / SNORM16).mul_add(half_of(axis), f32::midpoint(axis.0, axis.1));
    }
    out
}

fn tight(low: [i16; 3], high: [i16; 3], axes: Box3) -> bool {
    let (low, high) = (decode(low, axes), decode(high, axes));
    low.iter().zip(&high).zip(axes).all(|((low, high), axis)| {
        (low - axis.0).abs() <= tolerance(axis) && (high - axis.1).abs() <= tolerance(axis)
    })
}

fn contains(low: [i16; 3], high: [i16; 3], axes: Box3) -> bool {
    let (low, high) = (decode(low, axes), decode(high, axes));
    low.iter().zip(&high).zip(axes).all(|((low, high), axis)| {
        *low >= axis.0 - tolerance(axis) && *high <= axis.1 + tolerance(axis)
    })
}

fn boxes_equal(first: Box3, second: Box3) -> bool {
    first
        .iter()
        .zip(second)
        .all(|(a, b)| (a.0 - b.0).abs() <= 1e-4 && (a.1 - b.1).abs() <= 1e-4)
}

fn tally_chunk(tally: &mut Tally, path: &str, chunk: &[u8]) -> Option<()> {
    let first = box_at(chunk, 24)?;
    let second = box_at(chunk, 48)?;
    let (start, vertices) = quantised_vertices(chunk)?;
    let (low, high) = raw_extent(chunk, start, vertices)?;
    tally.chunks += 1;
    let (first_tight, second_tight) = (tight(low, high, first), tight(low, high, second));
    if boxes_equal(first, second) {
        tally.same += 1;
        tally.same_tight += u64::from(second_tight);
        return Some(());
    }
    tally.differ += 1;
    if tally.example_differing.is_empty() {
        tally.example_differing = format!("{path} first {first:?} second {second:?}");
    }
    tally.first_tight += u64::from(first_tight && !second_tight);
    tally.second_tight += u64::from(second_tight && !first_tight);
    tally.both_tight += u64::from(first_tight && second_tight);
    tally.neither_tight += u64::from(!first_tight && !second_tight);
    tally.first_contains += u64::from(contains(low, high, first));
    tally.second_contains += u64::from(contains(low, high, second));
    Some(())
}

fn scan(archive: &Archive, entry: &Entry) -> Survey {
    let mut survey = Survey::new();
    let mut bytes = Vec::new();
    let read = archive
        .open_entry(entry)
        .ok()
        .and_then(|reader| reader.take(MAX_FILE_LEN).read_to_end(&mut bytes).ok());
    let (Some(_), Ok(file)) = (read, ChunkFile::parse(bytes)) else {
        return survey;
    };
    let path = entry.path();
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    for (index, chunk) in file.chunks().iter().enumerate() {
        let kind = chunk.chunk_type();
        if kind != ivo::SKIN_2 && kind != ivo::SKIN {
            continue;
        }
        if let Some(data) = file.chunk_data(index) {
            let tally = survey.entry(extension.clone()).or_default();
            tally_chunk(tally, path, data);
        }
    }
    survey
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the survey tables are the measurement; the test prints them"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_two_boxes_of_the_mesh_chunks_are_compared_with_the_vertices_they_decode() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let survey = archive
        .entries()
        .par_iter()
        .filter(|entry| {
            let path = entry.path();
            entry.uncompressed_size() <= MAX_FILE_LEN
                && EXTENSIONS.iter().any(|extension| {
                    path.len() > extension.len()
                        && path[path.len() - extension.len()..].eq_ignore_ascii_case(extension)
                })
        })
        .map(|entry| scan(&archive, entry))
        .reduce(Survey::new, merge);
    for (extension, tally) in &survey {
        println!(
            ".{extension}: quantised chunks {}, equal boxes {} (tight {}), differing {} [only \
             first tight {}, only second tight {}, both {}, neither {}; first contains {}, second \
             contains {}]",
            tally.chunks,
            tally.same,
            tally.same_tight,
            tally.differ,
            tally.first_tight,
            tally.second_tight,
            tally.both_tight,
            tally.neither_tight,
            tally.first_contains,
            tally.second_contains,
        );
        println!("  differing example: {}", tally.example_differing);
    }
    assert!(!survey.is_empty(), "the Archive holds mesh chunks");
    assert!(
        survey.values().any(|tally| tally.chunks > 0),
        "the Archive holds quantised meshes"
    );
    for (extension, tally) in &survey {
        assert_eq!(
            tally.first_tight, 0,
            ".{extension}: the first box is never the only tight one"
        );
        assert_eq!(
            tally.second_contains, tally.differ,
            ".{extension}: the second box contains the decoded vertices wherever the boxes differ"
        );
        assert_eq!(
            tally.same_tight, tally.same,
            ".{extension}: equal boxes are tight"
        );
    }
    Ok(())
}
