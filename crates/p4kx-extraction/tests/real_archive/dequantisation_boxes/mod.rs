//! Survey of the two bounding boxes of the mesh chunks of a real Archive; runs with `--ignored`
//! when `P4KX_DATA_P4K` names one and prints the tables (`--nocapture`).
//!
//! Quantised positions are decoded with each of the two boxes. The box that reproduces the mesh is
//! the one whose positions give face normals that agree with the stored vertex normals, which a
//! wrong, unevenly scaled box distorts. That test does not depend on the boxes themselves.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::io::Read;
use std::path::Path;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, ChunkType, Container, ivo};
use p4kx_testkit::{MeshStreams, mesh_streams, real_archive_path};
use rayon::prelude::*;

mod math;

use math::{
    Box3, box_at, boxes_equal, cross, decode, dot, frame_normal, minus, normalise, raw_position,
    word,
};

type TestResult = Result<(), Box<dyn Error>>;

const COMPANIONS: [&str; 2] = ["cgfm", "cgam"];
const MAX_FILE_LEN: u64 = 512 << 20;
const INDICES: u32 = 0xEECD_C168;
const VERTS_UVS_2: u32 = 0xB3A7_0D5E;
const QTANGENTS: u32 = 0xEE05_7252;
const HEADER_LEN: usize = 168;
const SUBMESH_LEN: usize = 48;
const TRIANGLES_PER_SUBMESH: usize = 200;
const MARGIN: f32 = 0.02;
const CLEAR_MARGIN: f32 = 0.1;
const VIS_AREAS: u32 = 0xB324_59D2;

#[derive(Default)]
struct Tally {
    quantised: u64,
    floats: u64,
    equal: u64,
    differ: u64,
    first_better: u64,
    second_better: u64,
    tied: u64,
    undecidable: u64,
    first_clear: u64,
    second_clear: u64,
    visibility_areas: u64,
    info_is_first: u64,
    info_is_second: u64,
    unreadable: Vec<String>,
    example_first_better: String,
    example_second_better: String,
}

impl Tally {
    fn merge(&mut self, other: Self) {
        self.quantised += other.quantised;
        self.floats += other.floats;
        self.equal += other.equal;
        self.differ += other.differ;
        self.first_better += other.first_better;
        self.second_better += other.second_better;
        self.tied += other.tied;
        self.undecidable += other.undecidable;
        self.first_clear += other.first_clear;
        self.second_clear += other.second_clear;
        self.visibility_areas += other.visibility_areas;
        self.info_is_first += other.info_is_first;
        self.info_is_second += other.info_is_second;
        self.unreadable.extend(other.unreadable);
        for (mine, theirs) in [
            (&mut self.example_first_better, other.example_first_better),
            (&mut self.example_second_better, other.example_second_better),
        ] {
            if mine.is_empty() {
                *mine = theirs;
            }
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

struct Mesh<'a> {
    chunk: &'a [u8],
    walked: &'a MeshStreams,
}

impl Mesh<'_> {
    fn stream(&self, tag: u32, size: u32) -> Option<&[u8]> {
        let found = self
            .walked
            .streams
            .iter()
            .find(|stream| stream.tag == tag && stream.element_size == size)?;
        self.chunk.get(found.data.clone())
    }

    fn index(indices: &[u8], wide: bool, position: usize) -> Option<usize> {
        if wide {
            usize::try_from(word(indices, position.checked_mul(4)?)?).ok()
        } else {
            let at = position.checked_mul(2)?;
            let pair = <[u8; 2]>::try_from(indices.get(at..at + 2)?).ok()?;
            Some(usize::from(u16::from_le_bytes(pair)))
        }
    }

    fn score(&self, axes: Box3) -> Option<f32> {
        let positions = self.stream(VERTS_UVS_2, 16)?;
        let frames = self.stream(QTANGENTS, 8)?;
        let (indices, wide) = self
            .stream(INDICES, 2)
            .map(|data| (data, false))
            .or_else(|| self.stream(INDICES, 4).map(|data| (data, true)))?;
        let submeshes = usize::try_from(word(self.chunk, 16)?).ok()?;
        let (mut sum, mut count) = (0.0_f32, 0_u32);
        for submesh in 0..submeshes {
            let at = HEADER_LEN + submesh * SUBMESH_LEN;
            let first = usize::try_from(word(self.chunk, at + 4)?).ok()?;
            let total = usize::try_from(word(self.chunk, at + 8)?).ok()?;
            let base = usize::try_from(word(self.chunk, at + 16)?).ok()?;
            let triangles = total / 3;
            let step = (triangles / TRIANGLES_PER_SUBMESH).max(1);
            for triangle in (0..triangles).step_by(step) {
                let mut points = [[0.0; 3]; 3];
                let mut normals = [0.0; 3];
                for (corner, point) in points.iter_mut().enumerate() {
                    let position = first + triangle * 3 + corner;
                    let vertex = base + Self::index(indices, wide, position)?;
                    *point = decode(raw_position(positions, vertex)?, axes);
                    for (sum, normal) in normals.iter_mut().zip(frame_normal(frames, vertex)?) {
                        *sum += normal;
                    }
                }
                let [p0, p1, p2] = points;
                let face = cross(minus(p1, p0), minus(p2, p0));
                if dot(face, face) > 1e-18 {
                    sum += dot(normalise(face), normalise(normals));
                    count += 1;
                }
            }
        }
        #[expect(clippy::cast_precision_loss, reason = "a sample count far below 2^24")]
        (count > 0).then(|| sum / count as f32)
    }
}

fn read_chunk_file(archive: &Archive, entry: &Entry) -> Option<ChunkFile<Vec<u8>>> {
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .ok()?
        .take(MAX_FILE_LEN)
        .read_to_end(&mut bytes)
        .ok()?;
    ChunkFile::parse(bytes).ok()
}

fn first_data<'a>(file: &'a ChunkFile<Vec<u8>>, kinds: &[ChunkType]) -> Option<&'a [u8]> {
    let (index, _) = kinds.iter().find_map(|&kind| file.chunks_of(kind).next())?;
    file.chunk_data(index)
}

fn tally_mesh(tally: &mut Tally, path: &str, model: Option<&[u8]>, companion: &[u8]) -> Option<()> {
    let (first, second) = (box_at(companion, 24)?, box_at(companion, 48)?);
    let walked = mesh_streams(companion)?;
    let mesh = Mesh {
        chunk: companion,
        walked: &walked,
    };
    if mesh.stream(VERTS_UVS_2, 16).is_none() {
        tally.floats += 1;
        return Some(());
    }
    tally.quantised += 1;
    if let Some(info) = model.and_then(|model| box_at(model, 24)) {
        tally.info_is_first += u64::from(boxes_equal(info, first));
        tally.info_is_second += u64::from(boxes_equal(info, second));
    }
    if boxes_equal(first, second) {
        tally.equal += 1;
        return Some(());
    }
    tally.differ += 1;
    let (Some(score_first), Some(score_second)) = (mesh.score(first), mesh.score(second)) else {
        tally.undecidable += 1;
        return Some(());
    };
    let example = format!("{path} first {first:?} second {second:?}");
    let delta = score_second - score_first;
    tally.first_clear += u64::from(delta < -CLEAR_MARGIN);
    tally.second_clear += u64::from(delta > CLEAR_MARGIN);
    if score_second - score_first > MARGIN {
        tally.second_better += 1;
        if tally.example_second_better.is_empty() {
            tally.example_second_better = example;
        }
    } else if score_first - score_second > MARGIN {
        tally.first_better += 1;
        if tally.example_first_better.is_empty() {
            tally.example_first_better = example;
        }
    } else {
        tally.tied += 1;
    }
    Some(())
}

fn scan(archive: &Archive, index: &HashMap<String, &Entry>, entry: &Entry) -> Survey {
    let mut survey = Survey::new();
    let path = entry.path();
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let tally = survey.entry(extension).or_default();
    let model = index
        .get(&path[..path.len() - 1].to_ascii_lowercase())
        .and_then(|model| read_chunk_file(archive, model));
    let Some(companion) = read_chunk_file(archive, entry) else {
        tally.unreadable.push(path.to_owned());
        return survey;
    };
    let Some(skin) = first_data(&companion, &[ivo::SKIN_2, ivo::SKIN]) else {
        let vis_areas = ChunkType::new(Container::Ivo, VIS_AREAS);
        if companion.chunks_of(vis_areas).next().is_some() {
            tally.visibility_areas += 1;
        } else {
            tally.unreadable.push(path.to_owned());
        }
        return survey;
    };
    let info = model
        .as_ref()
        .and_then(|model| first_data(model, &[ivo::MESH_INFO]));
    if tally_mesh(tally, path, info, skin).is_none() {
        tally.unreadable.push(path.to_owned());
    }
    survey
}

fn has_extension(entry: &Entry, extensions: &[&str]) -> bool {
    Path::new(entry.path()).extension().is_some_and(|found| {
        extensions
            .iter()
            .any(|wanted| found.eq_ignore_ascii_case(wanted))
    })
}

#[expect(
    clippy::print_stdout,
    reason = "the survey tables are the measurement; the test prints them"
)]
fn report(survey: &Survey) {
    for (extension, tally) in survey {
        println!(
            ".{extension}: quantised {}, float {}, unreadable {} {:?}; equal boxes {}; differing \
             {} [face normals agree better with the first box {}, with the second {}, tied {}, \
             undecidable {}; clearly (> 0.1) first {}, second {}]; MeshInfo box = first box in {}, \
             = second in {}; without mesh chunk (VisAreas) {}",
            tally.quantised,
            tally.floats,
            tally.unreadable.len(),
            tally.unreadable.iter().take(3).collect::<Vec<_>>(),
            tally.equal,
            tally.differ,
            tally.first_better,
            tally.second_better,
            tally.tied,
            tally.undecidable,
            tally.first_clear,
            tally.second_clear,
            tally.info_is_first,
            tally.info_is_second,
            tally.visibility_areas,
        );
        println!("  first better: {}", tally.example_first_better);
        println!("  second better: {}", tally.example_second_better);
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_two_boxes_of_the_mesh_chunks_are_judged_by_the_face_normals_they_give() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index: HashMap<String, &Entry> = archive
        .entries()
        .iter()
        .filter(|entry| has_extension(entry, &["cgf", "cga"]))
        .map(|entry| (entry.path().to_ascii_lowercase(), entry))
        .collect();
    let survey = archive
        .entries()
        .par_iter()
        .filter(|entry| has_extension(entry, &COMPANIONS))
        .map(|entry| scan(&archive, &index, entry))
        .reduce(Survey::new, merge);
    report(&survey);
    assert!(
        survey.values().any(|tally| tally.quantised > 0),
        "the Archive holds quantised meshes"
    );
    for (extension, tally) in &survey {
        assert!(
            tally.unreadable.is_empty(),
            ".{extension}: unreadable {:#?}",
            tally.unreadable
        );
        assert_eq!(tally.undecidable, 0, ".{extension}: meshes without scores");
        assert!(
            tally.first_clear * 20 <= tally.second_clear,
            ".{extension}: the first box wins clearly in {} chunks, the second in {}",
            tally.first_clear,
            tally.second_clear
        );
    }
    let second_clear: u64 = survey.values().map(|tally| tally.second_clear).sum();
    assert!(second_clear > 0, "no differing chunk is decided");
    Ok(())
}
