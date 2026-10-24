// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Static models of a real Archive as GLB; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{Archive, Entry, GlbOptions, ModelSource, PathIndex};
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{CheckedGlb, check_glb, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const EXTENSIONS: [&str; 2] = [".cgf", ".cga"];
const SAMPLE_PER_EXTENSION: usize = 200;
const MIN_AGREEING_SHARE: f64 = 0.95;

#[derive(Default)]
struct Tally {
    exported: usize,
    triangles: u64,
    agreeing: u64,
    failures: Vec<String>,
}

impl Tally {
    fn merge(mut self, other: Self) -> Self {
        self.exported += other.exported;
        self.triangles += other.triangles;
        self.agreeing += other.agreeing;
        self.failures.extend(other.failures);
        self
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_static_models_exports_valid_glb_with_normals_along_the_winding() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let out = tempfile::tempdir()?;
    let mut sample: Vec<&Entry> = Vec::new();
    for extension in EXTENSIONS {
        let candidates: Vec<&Entry> = archive
            .entries()
            .iter()
            .filter(|entry| entry.path().to_ascii_lowercase().ends_with(extension))
            .collect();
        let stride = (candidates.len() / SAMPLE_PER_EXTENSION).max(1);
        sample.extend(
            candidates
                .into_iter()
                .step_by(stride)
                .take(SAMPLE_PER_EXTENSION),
        );
    }

    let tally = sample
        .par_iter()
        .enumerate()
        .map(|(position, entry)| export(&index, entry, &out.path().join(format!("{position}.glb"))))
        .reduce(Tally::default, Tally::merge);

    assert!(tally.exported > 0, "the Archive holds static models");
    assert!(tally.failures.is_empty(), "failed: {:#?}", tally.failures);
    #[expect(
        clippy::cast_precision_loss,
        reason = "triangle counts stay far below 2^52"
    )]
    let share = tally.agreeing as f64 / tally.triangles.max(1) as f64;
    assert!(
        share >= MIN_AGREEING_SHARE,
        "only {share:.3} of {} triangles have normals along their winding",
        tally.triangles
    );
    Ok(())
}

fn export(index: &PathIndex<'_>, entry: &Entry, out: &Path) -> Tally {
    let mut tally = Tally::default();
    let result = ModelSource::from_index(index, entry.path())
        .map_err(|error| format!("{error:#?}"))
        .and_then(|source| {
            source
                .write_glb(&GlbOptions::default(), out)
                .map_err(|error| format!("{error:#?}"))
        })
        .and_then(|_| fs::read(out).map_err(|error| error.to_string()))
        .and_then(|bytes| check_glb(&bytes));
    match result {
        Ok(glb) => {
            tally.exported = 1;
            let (triangles, agreeing) = winding(&glb);
            tally.triangles = triangles;
            tally.agreeing = agreeing;
        }
        Err(error) => tally.failures.push(format!("{}: {error}", entry.path())),
    }
    let _ignored = fs::remove_file(out);
    tally
}

fn winding(glb: &CheckedGlb) -> (u64, u64) {
    let mut triangles = 0;
    let mut agreeing = 0;
    for mesh in glb.document.meshes() {
        for primitive in mesh.primitives() {
            let (Some(positions), Some(normals), Some(indices)) = (
                primitive.get(&Semantic::Positions),
                primitive.get(&Semantic::Normals),
                primitive.indices(),
            ) else {
                continue;
            };
            let positions = glb.floats(&positions);
            let normals = glb.floats(&normals);
            for corners in glb.indices(&indices).as_chunks::<3>().0 {
                let [first, second, third] = corners.map(|corner| vector(&positions, corner));
                let face = cross(sub(second, first), sub(third, first));
                if dot(face, face) < 1e-12 {
                    continue;
                }
                let normal = corners
                    .map(|corner| vector(&normals, corner))
                    .into_iter()
                    .fold([0.0; 3], add);
                triangles += 1;
                if dot(face, normal) > 0.0 {
                    agreeing += 1;
                }
            }
        }
    }
    (triangles, agreeing)
}

fn vector(values: &[f32], index: u32) -> [f64; 3] {
    let at = index as usize * 3;
    values
        .get(at..at + 3)
        .and_then(|slice| <[f32; 3]>::try_from(slice).ok())
        .map_or([0.0; 3], |vector| vector.map(f64::from))
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|axis| left.get(axis).unwrap_or(&0.0) - right.get(axis).unwrap_or(&0.0))
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|axis| left.get(axis).unwrap_or(&0.0) + right.get(axis).unwrap_or(&0.0))
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.iter().zip(right).map(|(l, r)| l * r).sum()
}

fn cross([ax, ay, az]: [f64; 3], [bx, by, bz]: [f64; 3]) -> [f64; 3] {
    [
        ay.mul_add(bz, -(az * by)),
        az.mul_add(bx, -(ax * bz)),
        ax.mul_add(by, -(ay * bx)),
    ]
}
