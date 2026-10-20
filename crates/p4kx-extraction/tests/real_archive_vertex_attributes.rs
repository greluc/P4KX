//! Tangents and vertex colours of the meshes of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one (and `P4KX_DATA_P4K_B` for a second).

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use p4kx_extraction::model::Mesh;
use p4kx_extraction::{Archive, Entry, ModelSource, PathIndex};
use p4kx_testkit::{real_archive_b_path, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: usize = 300;
const TRIANGLES_PER_MESH: usize = 300;
const MIN_ALONG_SHARE: f64 = 0.95;
const MIN_HANDED_SHARE: f64 = 0.95;
const CLEAR_COSINE: f32 = 0.5;

#[derive(Default)]
struct Tally {
    meshes: u64,
    with_tangents: u64,
    with_colours: u64,
    corners: u64,
    along: u64,
    against: u64,
    handed: u64,
    handed_checked: u64,
    colours: BTreeMap<[u8; 4], u64>,
}

impl Tally {
    fn merge(mut self, other: Self) -> Self {
        self.meshes += other.meshes;
        self.with_tangents += other.with_tangents;
        self.with_colours += other.with_colours;
        self.corners += other.corners;
        self.along += other.along;
        self.against += other.against;
        self.handed += other.handed;
        self.handed_checked += other.handed_checked;
        for (colour, count) in other.colours {
            *self.colours.entry(colour).or_default() += count;
        }
        self
    }
}

type Vector = [f32; 3];

fn sub([ax, ay, az]: Vector, [bx, by, bz]: Vector) -> Vector {
    [ax - bx, ay - by, az - bz]
}

fn dot([ax, ay, az]: Vector, [bx, by, bz]: Vector) -> f32 {
    ax.mul_add(bx, ay.mul_add(by, az * bz))
}

fn cross([ax, ay, az]: Vector, [bx, by, bz]: Vector) -> Vector {
    [
        ay.mul_add(bz, -(az * by)),
        az.mul_add(bx, -(ax * bz)),
        ax.mul_add(by, -(ay * bx)),
    ]
}

fn combine([ax, ay, az]: Vector, scale_a: f32, [bx, by, bz]: Vector, scale_b: f32) -> Vector {
    [
        ax.mul_add(scale_a, -(bx * scale_b)),
        ay.mul_add(scale_a, -(by * scale_b)),
        az.mul_add(scale_a, -(bz * scale_b)),
    ]
}

/// Returns the directions of increasing U and V on the triangle `corners` of `mesh`, or `None`
/// for a triangle with degenerate texture coordinates.
fn texture_axes(mesh: &Mesh, corners: [usize; 3]) -> Option<(Vector, Vector)> {
    let [a, b, c] = corners;
    let (pa, pb, pc) = (
        *mesh.positions.get(a)?,
        *mesh.positions.get(b)?,
        *mesh.positions.get(c)?,
    );
    let ([ua, va], [ub, vb], [uc, vc]) = (*mesh.uvs.get(a)?, *mesh.uvs.get(b)?, *mesh.uvs.get(c)?);
    let (e1, e2) = (sub(pb, pa), sub(pc, pa));
    let (du1, dv1, du2, dv2) = (ub - ua, vb - va, uc - ua, vc - va);
    let det = du1.mul_add(dv2, -(du2 * dv1));
    if det.abs() < 1e-9 {
        return None;
    }
    let along_u = combine(e1, dv2, e2, dv1).map(|value| value / det);
    let along_v = combine(e2, du1, e1, du2).map(|value| value / det);
    (dot(along_u, along_u) > 1e-18 && dot(along_v, along_v) > 1e-18).then_some((along_u, along_v))
}

fn check_tangents(mesh: &Mesh, tangents: &[[f32; 4]], normals: &[Vector]) -> Tally {
    let mut tally = Tally::default();
    for corners in mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .take(TRIANGLES_PER_MESH)
    {
        let corners = corners.map(|corner| corner as usize);
        let Some((along_u, along_v)) = texture_axes(mesh, corners) else {
            continue;
        };
        let length = dot(along_u, along_u).sqrt();
        for vertex in corners {
            let (Some(&[x, y, z, handedness]), Some(&normal)) =
                (tangents.get(vertex), normals.get(vertex))
            else {
                continue;
            };
            let cosine = dot([x, y, z], along_u) / length;
            tally.corners += 1;
            if cosine < -CLEAR_COSINE {
                tally.against += 1;
            } else if cosine > CLEAR_COSINE {
                tally.along += 1;
                tally.handed_checked += 1;
                let bitangent_along_v = dot(cross(normal, [x, y, z]), along_v) > 0.0;
                tally.handed += u64::from(bitangent_along_v == (handedness > 0.0));
            }
        }
    }
    tally
}

fn scan(index: &PathIndex<'_>, entry: &Entry) -> Tally {
    let mut tally = Tally::default();
    let Ok(source) = ModelSource::from_index(index, entry.path()) else {
        return tally;
    };
    for mesh in source.model().lods.iter().flat_map(|lod| &lod.meshes) {
        tally.meshes += 1;
        if let Some(colours) = &mesh.colours {
            tally.with_colours += 1;
            for &colour in colours {
                *tally.colours.entry(colour).or_default() += 1;
            }
        }
        if let (Some(tangents), Some(normals)) = (&mesh.tangents, &mesh.normals) {
            tally.with_tangents += 1;
            tally = tally.merge(check_tangents(mesh, tangents, normals));
        }
    }
    tally
}

fn archives() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths = vec![real_archive_path()?];
    paths.extend(real_archive_b_path());
    Ok(paths)
}

fn survey(archive: &Archive, suffix: &str) -> Tally {
    let index = PathIndex::new(archive);
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(suffix))
        .collect();
    let stride = (candidates.len() / SAMPLE).max(1);
    candidates
        .into_iter()
        .step_by(stride)
        .take(SAMPLE)
        .collect::<Vec<_>>()
        .par_iter()
        .map(|entry| scan(&index, entry))
        .reduce(Tally::default, Tally::merge)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn tangents_run_along_the_u_axis_and_their_handedness_gives_the_v_axis() -> TestResult {
    for path in archives()? {
        let archive = Archive::open(&path)?;
        for suffix in [".cgf", ".cga", ".skin"] {
            let tally = survey(&archive, suffix);
            assert!(
                tally.with_tangents > 50,
                "{} {suffix}: {} meshes with tangents",
                path.display(),
                tally.with_tangents
            );
            #[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
            let along = tally.along as f64 / (tally.along + tally.against).max(1) as f64;
            #[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
            let handed = tally.handed as f64 / tally.handed_checked.max(1) as f64;
            assert!(
                along >= MIN_ALONG_SHARE && handed >= MIN_HANDED_SHARE,
                "{} {suffix}: tangent along the U axis at {along:.3} of {} corners, handedness \
                 right at {handed:.3} of {}",
                path.display(),
                tally.along + tally.against,
                tally.handed_checked
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn meshes_with_vertex_colours_hold_more_than_opaque_white() -> TestResult {
    for path in archives()? {
        let archive = Archive::open(&path)?;
        for suffix in [".cgf", ".cga", ".skin"] {
            let tally = survey(&archive, suffix);
            assert!(
                tally.with_colours > 10 && tally.with_colours < tally.meshes,
                "{} {suffix}: {} of {} meshes have colours",
                path.display(),
                tally.with_colours,
                tally.meshes
            );
            assert!(
                tally.colours.len() > 5,
                "{} {suffix}: {} distinct colours",
                path.display(),
                tally.colours.len()
            );
        }
    }
    Ok(())
}
