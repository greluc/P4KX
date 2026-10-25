// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Computed normals, the model box, lenient levels of detail and the node fields on ships and an
//! interior CGF of a real Archive (ADR-0119); runs with `--ignored` when `P4KX_DATA_P4K` names
//! one and prints the tallies (`--nocapture`).
#![expect(
    clippy::print_stdout,
    reason = "the survey tests print their tallies for --nocapture"
)]

use std::error::Error;
use std::path::Path;

use p4kx_extraction::model::{LodSelection, Model, ModelIssue, PositionBox};
use p4kx_extraction::{Archive, Entry, ModelSource, PathIndex};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: &str = "objects\\spaceships\\ships\\";
const SHIP_SAMPLE: usize = 80;
const WIDE_SAMPLE: usize = 3000;
const INTERIOR_SAMPLE: usize = 400;
const UNIT_TOLERANCE: f32 = 1e-3;
const CLEAR_MARGIN: f32 = 0.02;
const TRIANGLES_PER_MESH: usize = 200;

fn sample(archive: &Archive, keep: impl Fn(&str) -> bool, count: usize) -> Vec<&Entry> {
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| keep(&entry.path().to_ascii_lowercase()))
        .collect();
    let stride = (candidates.len() / count).max(1);
    candidates.into_iter().step_by(stride).take(count).collect()
}

fn has_extension(path: &str, extensions: &[&str]) -> bool {
    Path::new(path).extension().is_some_and(|found| {
        extensions
            .iter()
            .any(|extension| found.eq_ignore_ascii_case(extension))
    })
}

fn is_ship_model(path: &str) -> bool {
    path.contains(SHIPS) && has_extension(path, &["cga", "cgf"])
}

#[derive(Default)]
struct NodeTally {
    models: u64,
    nodes: u64,
    helpers: u64,
    geometry: u64,
    scaled: u64,
    with_properties: u64,
    with_class: u64,
    with_material_indices: u64,
    meshes: u64,
    computed: u64,
    failures: Vec<String>,
}

impl NodeTally {
    fn summary(&self) -> String {
        format!(
            "{} models: {} nodes ({} geometry, {} helpers, {} scaled, {} with properties, \
             {} with a class), {} with material indices; {} meshes, {} with computed normals",
            self.models,
            self.nodes,
            self.geometry,
            self.helpers,
            self.scaled,
            self.with_properties,
            self.with_class,
            self.with_material_indices,
            self.meshes,
            self.computed
        )
    }

    fn merge(mut self, other: Self) -> Self {
        self.models += other.models;
        self.nodes += other.nodes;
        self.helpers += other.helpers;
        self.geometry += other.geometry;
        self.scaled += other.scaled;
        self.with_properties += other.with_properties;
        self.with_class += other.with_class;
        self.with_material_indices += other.with_material_indices;
        self.meshes += other.meshes;
        self.computed += other.computed;
        self.failures.extend(other.failures);
        self
    }
}

fn tally_model(index: &PathIndex<'_>, path: &str) -> NodeTally {
    let mut tally = NodeTally::default();
    let source = match ModelSource::from_index_without_materials(index, path, LodSelection::All) {
        Ok(source) => source,
        Err(error) => {
            tally.failures.push(format!("{path}: {error:?}"));
            return tally;
        }
    };
    let model = source.model();
    tally.models = 1;
    tally.with_material_indices = u64::from(!model.material_indices.is_empty());
    for node in &model.nodes {
        tally.nodes += 1;
        tally.helpers += u64::from(node.is_helper());
        tally.geometry += u64::from(node.geometry_type == Some(0));
        tally.scaled += u64::from(node.scale.iter().any(|value| (value - 1.0).abs() > 1e-6));
        tally.with_properties += u64::from(!node.properties.is_empty());
        tally.with_class += u64::from(node.property("class").is_some());
        if node.scale.iter().any(|value| !value.is_finite()) {
            tally
                .failures
                .push(format!("{path}: scale {:?}", node.scale));
        }
    }
    for mesh in model.lods.iter().flat_map(|lod| &lod.meshes) {
        tally.meshes += 1;
        tally.computed += u64::from(mesh.normals_computed);
        let Some(normals) = &mesh.normals else {
            tally
                .failures
                .push(format!("{path}: a mesh without normals"));
            continue;
        };
        if normals.len() != mesh.positions.len() {
            tally.failures.push(format!("{path}: normal count"));
        }
        let bad = normals.iter().any(|[x, y, z]| {
            (x.mul_add(*x, y.mul_add(*y, z * z)).sqrt() - 1.0).abs() > UNIT_TOLERANCE
        });
        if bad {
            tally
                .failures
                .push(format!("{path}: a normal is not a unit vector"));
        }
    }
    tally
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn ship_models_have_unit_normals_and_carry_the_node_fields() -> TestResult {
    let archive = Archive::open(&real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let paths: Vec<&str> = sample(&archive, is_ship_model, SHIP_SAMPLE)
        .into_iter()
        .map(Entry::path)
        .collect();
    let tally = paths
        .par_iter()
        .map(|path| tally_model(&index, path))
        .reduce(NodeTally::default, NodeTally::merge);
    println!("{}", tally.summary());
    assert!(tally.failures.is_empty(), "{:#?}", tally.failures);
    assert!(tally.models > 20, "{} models", tally.models);
    assert!(tally.geometry > 0 && tally.helpers > 0, "node types");
    assert!(tally.with_material_indices > 0, "material indices");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_physics_mesh_without_stored_normals_gets_smooth_ones() -> TestResult {
    let archive = Archive::open(&real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let entry = archive
        .entries()
        .iter()
        .find(|entry| {
            !entry.is_encrypted()
                && entry
                    .path()
                    .to_ascii_lowercase()
                    .ends_with("anvl_carrack_no_colide.vvg")
        })
        .ok_or("the Carrack physics proxy")?;
    let source = ModelSource::from_index(&index, entry.path())?;
    let mesh = source
        .model()
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .ok_or("a mesh")?;
    assert!(mesh.normals_computed);
    let normals = mesh.normals.as_ref().ok_or("normals")?;
    assert_eq!(normals.len(), 913);
    assert!(normals.iter().all(|[x, y, z]| {
        (x.mul_add(*x, y.mul_add(*y, z * z)).sqrt() - 1.0).abs() < UNIT_TOLERANCE
    }));
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn models_across_the_archive_have_unit_normals_and_carry_the_node_fields() -> TestResult {
    let archive = Archive::open(&real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let any_model = |path: &str| {
        [".cgf", ".cga", ".skin", ".chr"]
            .iter()
            .any(|suffix| path.ends_with(suffix))
    };
    let paths: Vec<&str> = sample(&archive, any_model, WIDE_SAMPLE)
        .into_iter()
        .map(Entry::path)
        .collect();
    let tally = paths
        .par_iter()
        .map(|path| tally_model(&index, path))
        .reduce(NodeTally::default, NodeTally::merge);
    println!("{}", tally.summary());
    assert!(tally.failures.is_empty(), "{:#?}", tally.failures);
    assert!(tally.models > 500, "{} models", tally.models);
    assert!(tally.with_class > 0, "no node property class=…");
    assert!(tally.scaled > 0, "no node with a scale");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_missing_level_falls_back_to_the_base_meshes_and_an_existing_one_does_not() -> TestResult {
    let archive = Archive::open(&real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let mut checked = 0_u32;
    let mut with_level = 0_u32;
    for entry in sample(&archive, is_ship_model, SHIP_SAMPLE) {
        let path = entry.path();
        let base = ModelSource::from_index_without_materials(&index, path, LodSelection::Only(0))?;
        let Some(base_lod) = base.model().lods.first() else {
            continue;
        };
        let lenient =
            ModelSource::from_index_without_materials(&index, path, LodSelection::OnlyOrBase(14))?;
        let model = lenient.model();
        let [lod] = model.lods.as_slice() else {
            return Err(format!("{path}: {} levels", model.lods.len()).into());
        };
        assert_eq!(lod.level, 14, "{path}");
        assert_eq!(lod.triangle_count(), base_lod.triangle_count(), "{path}");
        assert!(
            matches!(
                model.issues.as_slice(),
                [ModelIssue::LodFallback { level: 14, .. }]
            ),
            "{path}: {:?}",
            model.issues
        );
        checked += 1;
        let lod1 = format!(
            "{}_lod1.{}",
            &path[..path.len() - 4],
            &path[path.len() - 3..]
        );
        if index.find(&lod1).is_some() {
            let one = ModelSource::from_index_without_materials(
                &index,
                path,
                LodSelection::OnlyOrBase(1),
            )?;
            assert!(
                one.model().issues.is_empty(),
                "{path}: {:?}",
                one.model().issues
            );
            assert_eq!(one.model().lods.first().map(|lod| lod.level), Some(1));
            with_level += 1;
        }
    }
    println!("{checked} ships fell back, {with_level} had a real LOD1");
    assert!(checked > 20, "{checked} models");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_model_box_changes_the_positions_of_a_quantised_interior_cgf() -> TestResult {
    let archive = Archive::open(&real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let interior = |path: &str| {
        has_extension(path, &["cgf"])
            && (path.contains("interior") || path.contains("objectcontainers"))
    };
    let mut differing = None;
    let (mut read, mut scaling_clear, mut model_clear) = (0_u32, 0_u32, 0_u32);
    for entry in sample(&archive, interior, INTERIOR_SAMPLE) {
        let path = entry.path();
        let (Ok(scaling), Ok(model_box)) = (
            ModelSource::from_index_with_box(
                &index,
                path,
                LodSelection::Only(0),
                PositionBox::Scaling,
            ),
            ModelSource::from_index_with_box(
                &index,
                path,
                LodSelection::Only(0),
                PositionBox::Model,
            ),
        ) else {
            continue;
        };
        read += 1;
        let positions = |source: &ModelSource| {
            source
                .model()
                .lods
                .iter()
                .flat_map(|lod| &lod.meshes)
                .flat_map(|mesh| mesh.positions.iter().copied())
                .collect::<Vec<_>>()
        };
        let (a, b) = (positions(&scaling), positions(&model_box));
        assert_eq!(a.len(), b.len(), "{path}");
        assert!(b.iter().flatten().all(|value| value.is_finite()), "{path}");
        if a != b {
            differing.get_or_insert_with(|| path.to_owned());
            let (scaling_fit, model_fit) = (fit(scaling.model()), fit(model_box.model()));
            if scaling_fit - model_fit > CLEAR_MARGIN {
                scaling_clear += 1;
            } else if model_fit - scaling_fit > CLEAR_MARGIN {
                model_clear += 1;
            }
        }
    }
    println!(
        "{read} interior CGFs read both ways; first that differs: {differing:?}; by face-normal \
         fit the scaling box wins clearly in {scaling_clear}, the model box in {model_clear}"
    );
    assert!(read > 10, "{read} interior CGFs");
    assert!(differing.is_some(), "no interior CGF changes with the box");
    assert!(
        model_clear * 20 <= scaling_clear,
        "the model box fits clearly better in {model_clear}, the scaling box in {scaling_clear}"
    );
    Ok(())
}

fn fit(model: &Model) -> f32 {
    let (mut sum, mut count) = (0.0_f32, 0.0_f32);
    for mesh in model.lods.iter().flat_map(|lod| &lod.meshes) {
        let Some(normals) = mesh.normals.as_ref().filter(|_| !mesh.normals_computed) else {
            continue;
        };
        for triangle in mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .take(TRIANGLES_PER_MESH)
        {
            let at = |corner: u32| usize::try_from(corner).ok();
            let corners = triangle.map(|corner| {
                at(corner).and_then(|at| Some((*mesh.positions.get(at)?, *normals.get(at)?)))
            });
            let [Some(first), Some(second), Some(third)] = corners else {
                continue;
            };
            let edge1 = sub(second.0, first.0);
            let edge2 = sub(third.0, first.0);
            let face = [
                edge1[1].mul_add(edge2[2], -(edge1[2] * edge2[1])),
                edge1[2].mul_add(edge2[0], -(edge1[0] * edge2[2])),
                edge1[0].mul_add(edge2[1], -(edge1[1] * edge2[0])),
            ];
            let length = dot(face, face).sqrt();
            let (n1, n2, n3) = (first.1, second.1, third.1);
            let mean = [
                n1[0] + n2[0] + n3[0],
                n1[1] + n2[1] + n3[1],
                n1[2] + n2[2] + n3[2],
            ];
            let mean_length = dot(mean, mean).sqrt();
            if length > 1e-9 && mean_length > 1e-9 {
                sum += dot(face, mean) / (length * mean_length);
                count += 1.0;
            }
        }
    }
    if count == 0.0 { 0.0 } else { sum / count }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}
