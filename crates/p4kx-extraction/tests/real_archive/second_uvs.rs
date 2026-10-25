// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The four-byte `NORMALS2` stream of the mesh chunks of a real Archive; runs with `--ignored`
//! when `P4KX_DATA_P4K` names one (and `P4KX_DATA_P4K_B` for a second). The tables are printed
//! (`--nocapture`).

use std::error::Error;
use std::io::Read;
use std::path::Path;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_extraction::{ModelSource, PathIndex};
use p4kx_testkit::{MeshStream, mesh_streams, real_archive_b_path, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;
type Uv = [f32; 2];

const EXTENSIONS: [&str; 3] = ["cgfm", "cgam", "skinm"];
const MODEL_EXTENSIONS: [&str; 3] = ["cgf", "cga", "skin"];
const MAX_FILE_LEN: u64 = 512 << 20;
const MODEL_SAMPLE: usize = 600;
const TRIANGLES_PER_CHUNK: usize = 2000;
const NORMALS_2: u32 = 0x38A5_81FE;
const VERTS_UVS: u32 = 0x9132_9AE9;
const VERTS_UVS_2: u32 = 0xB3A7_0D5E;
const INDICES: u32 = 0xEECD_C168;
const OTHER_NORMALS: [(u32, u32); 5] = [
    (0x9CF3_F615, 4),
    (0x9CF3_F615, 12),
    (NORMALS_2, 12),
    (0xEE05_7252, 8),
    (0xB95E_9A1B, 8),
];

#[derive(Default)]
struct Tally {
    chunks: u64,
    sole_normal: u64,
    vertices: u64,
    finite: u64,
    in_unit: u64,
    in_range: u64,
    first_in_unit: u64,
    triangles: u64,
    triangles_with_area: u64,
}

impl Tally {
    const fn merge(mut self, other: &Self) -> Self {
        self.chunks += other.chunks;
        self.sole_normal += other.sole_normal;
        self.vertices += other.vertices;
        self.finite += other.finite;
        self.in_unit += other.in_unit;
        self.in_range += other.in_range;
        self.first_in_unit += other.first_in_unit;
        self.triangles += other.triangles;
        self.triangles_with_area += other.triangles_with_area;
        self
    }
}

#[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
fn share(part: u64, of: u64) -> f64 {
    part as f64 / of.max(1) as f64
}

fn has_extension(name: &str, extensions: &[&str]) -> bool {
    Path::new(name)
        .extension()
        .and_then(|found| found.to_str())
        .is_some_and(|found| {
            extensions
                .iter()
                .any(|extension| found.eq_ignore_ascii_case(extension))
        })
}

fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1F);
    let mantissa = f32::from(bits & 0x3FF);
    match exponent {
        0 => sign * mantissa * 2.0_f32.powi(-24),
        31 if mantissa == 0.0 => sign * f32::INFINITY,
        31 => f32::NAN,
        _ => sign * (1.0 + mantissa / 1024.0) * 2.0_f32.powi(exponent - 15),
    }
}

fn pair(bytes: [u8; 4]) -> Uv {
    let [a, b, c, d] = bytes;
    [
        half(u16::from_le_bytes([a, b])),
        half(u16::from_le_bytes([c, d])),
    ]
}

fn second_set(data: &[u8]) -> Vec<Uv> {
    data.as_chunks::<4>().0.iter().copied().map(pair).collect()
}

fn first_set(data: &[u8], stride: usize) -> Vec<Uv> {
    data.chunks_exact(stride)
        .filter_map(|vertex| vertex.last_chunk::<4>().copied())
        .map(pair)
        .collect()
}

fn index_list(data: &[u8], wide: bool) -> Vec<usize> {
    if wide {
        data.as_chunks::<4>()
            .0
            .iter()
            .map(|word| usize::try_from(u32::from_le_bytes(*word)).unwrap_or(usize::MAX))
            .collect()
    } else {
        data.as_chunks::<2>()
            .0
            .iter()
            .map(|word| usize::from(u16::from_le_bytes(*word)))
            .collect()
    }
}

fn in_unit(uv: Uv) -> bool {
    uv.iter().all(|value| (-0.001..=1.001).contains(value))
}

fn area(a: Uv, b: Uv, c: Uv) -> f32 {
    let [ax, ay] = a;
    let [bx, by] = b;
    let [cx, cy] = c;
    (bx - ax).mul_add(cy - ay, -((cx - ax) * (by - ay))).abs()
}

fn triangles_with_area(indices: &[usize], uvs: &[Uv]) -> (u64, u64) {
    let (mut all, mut with_area) = (0, 0);
    for triangle in indices.as_chunks::<3>().0.iter().take(TRIANGLES_PER_CHUNK) {
        let corners: Option<Vec<Uv>> = triangle
            .iter()
            .map(|&corner| uvs.get(corner).copied())
            .collect();
        if let Some(&[a, b, c]) = corners.as_deref() {
            all += 1;
            with_area += u64::from(area(a, b, c) > 1e-9);
        }
    }
    (all, with_area)
}

fn find(streams: &[MeshStream], wanted: impl Fn(&MeshStream) -> bool) -> Option<&MeshStream> {
    streams.iter().find(|stream| wanted(stream))
}

fn scan_chunk(chunk: &[u8]) -> Tally {
    let mut tally = Tally::default();
    let Some(walked) = mesh_streams(chunk) else {
        return tally;
    };
    let streams = walked.streams.as_slice();
    let Some(second) = find(streams, |s| s.tag == NORMALS_2 && s.element_size == 4) else {
        return tally;
    };
    tally.chunks = 1;
    tally.sole_normal = u64::from(
        !streams
            .iter()
            .any(|s| OTHER_NORMALS.contains(&(s.tag, s.element_size))),
    );
    let uvs = chunk.get(second.data.clone()).map(second_set);
    let uvs = uvs.unwrap_or_default();
    let first = find(streams, |s| {
        (s.tag == VERTS_UVS || s.tag == VERTS_UVS_2) && matches!(s.element_size, 16 | 20)
    })
    .and_then(|s| {
        Some(first_set(
            chunk.get(s.data.clone())?,
            s.element_size as usize,
        ))
    })
    .unwrap_or_default();
    for (at, uv) in uvs.iter().enumerate() {
        tally.vertices += 1;
        tally.finite += u64::from(uv.iter().all(|value| value.is_finite()));
        tally.in_unit += u64::from(in_unit(*uv));
        tally.in_range += u64::from(uv.iter().all(|value| (-64.0..=64.0).contains(value)));
        tally.first_in_unit += u64::from(first.get(at).copied().is_some_and(in_unit));
    }
    let list = find(streams, |s| s.tag == INDICES)
        .and_then(|s| Some(index_list(chunk.get(s.data.clone())?, s.element_size == 4)))
        .unwrap_or_default();
    (tally.triangles, tally.triangles_with_area) = triangles_with_area(&list, &uvs);
    tally
}

fn scan(archive: &Archive, entry: &Entry) -> Tally {
    let mut bytes = Vec::new();
    if archive
        .open_entry(entry)
        .ok()
        .and_then(|reader| reader.take(MAX_FILE_LEN).read_to_end(&mut bytes).ok())
        .is_none()
    {
        return Tally::default();
    }
    let Ok(file) = ChunkFile::parse(bytes) else {
        return Tally::default();
    };
    let mut tally = Tally::default();
    for (index, chunk) in file.chunks().iter().enumerate() {
        let kind = chunk.chunk_type();
        if kind != ivo::SKIN_2 && kind != ivo::SKIN {
            continue;
        }
        if let Some(data) = file.chunk_data(index) {
            tally = tally.merge(&scan_chunk(data));
        }
    }
    tally
}

fn open(path: &Path) -> Result<Archive, Box<dyn Error>> {
    Ok(Archive::open_with_key(
        path,
        Some(p4kx_testkit::real_archive_key()?),
    )?)
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the survey table is the measurement; the test prints it"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_four_byte_second_normal_stream_holds_half_float_texture_coordinates() -> TestResult {
    let mut paths = vec![real_archive_path()?];
    paths.extend(real_archive_b_path());
    for path in paths {
        let archive = open(&path)?;
        let entries: Vec<&Entry> = archive
            .entries()
            .iter()
            .filter(|entry| has_extension(entry.path(), &EXTENSIONS))
            .collect();
        let tally = entries
            .par_iter()
            .map(|entry| scan(&archive, entry))
            .reduce(Tally::default, |a, b| a.merge(&b));
        println!(
            "{}: chunks {} (sole normal source {}), vertices {}, finite {}, within 0..1 {}, \
             within -64..64 {}, first set within 0..1 {}, triangles {} with area {}",
            path.display(),
            tally.chunks,
            tally.sole_normal,
            tally.vertices,
            tally.finite,
            tally.in_unit,
            tally.in_range,
            tally.first_in_unit,
            tally.triangles,
            tally.triangles_with_area
        );
        let name = path.display();
        let of = tally.vertices;
        assert!(tally.chunks > 1000, "{name}: {} chunks", tally.chunks);
        assert_eq!(tally.sole_normal, 0, "{name}: a chunk with no other normal");
        assert!(share(tally.finite, of) > 0.9999, "{name}: finite values");
        assert!(
            share(tally.in_range, of) > 0.95,
            "{name}: texture-like values"
        );
        assert!(share(tally.in_unit, of) > 0.9, "{name}: values inside 0..1");
    }
    Ok(())
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the survey table is the measurement; the test prints it"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_model_reader_gives_the_second_texture_coordinates_per_vertex() -> TestResult {
    let archive = open(&real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| has_extension(entry.path(), &MODEL_EXTENSIONS))
        .collect();
    let stride = (candidates.len() / MODEL_SAMPLE).max(1);
    let (with, without) = candidates
        .into_iter()
        .step_by(stride)
        .take(MODEL_SAMPLE)
        .collect::<Vec<_>>()
        .par_iter()
        .map(|entry| {
            let mut counts = (0_u64, 0_u64);
            let Ok(source) = ModelSource::from_index(&index, entry.path()) else {
                return counts;
            };
            for mesh in source.model().lods.iter().flat_map(|lod| &lod.meshes) {
                if let Some(uvs) = &mesh.second_uvs {
                    assert_eq!(
                        uvs.len(),
                        mesh.positions.len(),
                        "{}: one per vertex",
                        entry.path()
                    );
                    counts.0 += 1;
                } else {
                    counts.1 += 1;
                }
            }
            counts
        })
        .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    println!("meshes with a second set {with}, without {without}");
    assert!(with > 0 && without > 0, "{with} with, {without} without");
    Ok(())
}
