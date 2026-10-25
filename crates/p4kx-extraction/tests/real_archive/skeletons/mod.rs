// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Skeletons and bone weights of the skins, characters and animated models of a real Archive;
//! runs with `--ignored` when `P4KX_DATA_P4K` names one. No entry involved is encrypted, so no
//! Archive key is needed. The tables are printed (`--nocapture`).

use std::error::Error;
use std::io::Read;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_extraction::{GlbOptions, ModelSource, PathIndex};
use p4kx_testkit::{check_glb, mesh_streams, real_archive_path};
use rayon::prelude::*;

mod support;

use support::{Tally, bind_difference, check_bones, check_skin, sample};

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn inverse_binds_from_the_chain_differ_little_from_the_stored_world_poses() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let entries: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            std::path::Path::new(entry.path())
                .extension()
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("skin") || ext.eq_ignore_ascii_case("chr")
                })
        })
        .collect();
    let (models, translation, rotation) = entries
        .par_iter()
        .filter_map(|entry| ModelSource::from_index(&index, entry.path()).ok())
        .map(|source| {
            let model = source.model();
            let mut largest = bind_difference(&model.bones);
            for lod in &model.lods {
                if let Some(bones) = &lod.bones {
                    let own = bind_difference(bones);
                    largest = (largest.0.max(own.0), largest.1.max(own.1));
                }
            }
            (1_u64, largest.0, largest.1)
        })
        .reduce(
            || (0, 0.0, 0.0),
            |a, b| (a.0 + b.0, a.1.max(b.1), a.2.max(b.2)),
        );
    println!(
        "{models} of {} skins and skeleton files: largest difference {translation:e} m in the \
         translation, {rotation:e} in a rotation component",
        entries.len()
    );
    assert!(models > 0, "the Archive holds skins and skeleton files");
    Ok(())
}

type TestResult = Result<(), Box<dyn Error>>;

const SKIN_SAMPLE: usize = 500;
const ANIMATED_SAMPLE: usize = 300;
const EXPORT_SAMPLE: usize = 150;
const MAX_FILE_LEN: u64 = 512 << 20;
const VERTS_UVS: u32 = 0x9132_9AE9;
const MIN_NEAR_SHARE: f64 = 0.9;

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn skins_have_consistent_bones_and_weights_that_sit_near_their_bone() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let mut entries = sample(&archive, ".skin", SKIN_SAMPLE);
    entries.extend(sample(&archive, "_head_skel.chr", 20));
    let tally = entries
        .par_iter()
        .map(|entry| check_skin(&index, entry.path()))
        .reduce(Tally::default, Tally::merge);

    #[expect(
        clippy::cast_precision_loss,
        reason = "vertex counts stay far below 2^52"
    )]
    let share = tally.near as f64 / (tally.near + tally.far).max(1) as f64;
    println!(
        "{} models, {} bones, {} levels with a skeleton of their own, {} skinned meshes, \
         {} vertices ({} with more than four bones), {share:.3} of the vertices are nearer to \
         their strongest bone than the median bone",
        tally.models,
        tally.bones,
        tally.level_skeletons,
        tally.skinned_meshes,
        tally.vertices,
        tally.over_four
    );
    assert!(tally.skinned_meshes > 0, "the sample holds skinned meshes");
    assert!(tally.failures.is_empty(), "failed: {:#?}", tally.failures);
    assert!(
        share >= MIN_NEAR_SHARE,
        "only {share:.3} of the vertices sit near their bone"
    );
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_skin_reads_with_every_level_weighted_to_bones_its_skeleton_has() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let entries: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".skin"))
        .collect();
    let (skins, levels, failures): (u64, u64, Vec<String>) = entries
        .par_iter()
        .map(
            |entry| match ModelSource::from_index(&index, entry.path()) {
                Ok(source) => {
                    let own = source
                        .model()
                        .lods
                        .iter()
                        .filter(|lod| lod.bones.is_some())
                        .count();
                    (1, own as u64, Vec::new())
                }
                Err(error) => (0, 0, vec![format!("{}: {error:?}", entry.path())]),
            },
        )
        .reduce(
            || (0, 0, Vec::new()),
            |(skins, levels, mut failures), (more, own, errors)| {
                failures.extend(errors);
                (skins + more, levels + own, failures)
            },
        );

    println!(
        "{skins} of {} skins read, {levels} levels with a skeleton of their own",
        entries.len()
    );
    assert!(failures.is_empty(), "failed: {failures:#?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_skeleton_file_has_bones_and_no_mesh_and_exports_as_nodes() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let out = tempfile::tempdir()?;
    let entries: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".chr"))
        .collect();
    let failures: Vec<String> = entries
        .par_iter()
        .enumerate()
        .filter_map(|(position, entry)| {
            let path = entry.path();
            let source = match ModelSource::from_index(&index, path) {
                Ok(source) => source,
                Err(error) => return Some(format!("{path}: {error:?}")),
            };
            let model = source.model();
            if model.bones.is_empty() || model.lods.iter().any(|lod| !lod.meshes.is_empty()) {
                return Some(format!("{path}: bones {} or a mesh", model.bones.len()));
            }
            let mut tally = Tally::default();
            check_bones(&model.bones, path, &mut tally);
            if let Some(failure) = tally.failures.into_iter().next() {
                return Some(failure);
            }
            let file = out.path().join(format!("{position}.glb"));
            let exported = source
                .write_glb(&GlbOptions::default(), &file)
                .map_err(|error| format!("{error:?}"))
                .and_then(|summary| {
                    (summary.bones == model.bones.len())
                        .then_some(())
                        .ok_or_else(|| format!("{} bone nodes", summary.bones))
                })
                .and_then(|()| std::fs::read(&file).map_err(|error| error.to_string()))
                .and_then(|bytes| check_glb(&bytes).map(|_| ()));
            let _ignored = std::fs::remove_file(&file);
            exported.err().map(|error| format!("{path}: {error}"))
        })
        .collect();

    assert!(!entries.is_empty(), "the Archive holds skeleton files");
    assert!(failures.is_empty(), "failed: {failures:#?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn skins_export_as_valid_glb_with_every_bone_as_a_joint() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let out = tempfile::tempdir()?;
    let entries = sample(&archive, ".skin", EXPORT_SAMPLE);
    let failures: Vec<String> = entries
        .par_iter()
        .enumerate()
        .filter_map(|(position, entry)| {
            let path = entry.path();
            let file = out.path().join(format!("{position}.glb"));
            let result = ModelSource::from_index(&index, path)
                .map_err(|error| format!("{error:?}"))
                .and_then(|source| {
                    for lod in &source.model().lods {
                        let options = GlbOptions::default().with_lod(lod.level);
                        let summary = source
                            .write_glb(&options, &file)
                            .map_err(|error| format!("{error:?}"))?;
                        let bytes = std::fs::read(&file).map_err(|error| error.to_string())?;
                        let glb = check_glb(&bytes)?;
                        let joints = glb
                            .document
                            .skins()
                            .next()
                            .map(|skin| skin.joints().count());
                        let bones = source.model().bones_of(lod).len();
                        let skinned = lod.meshes.iter().any(|mesh| mesh.influences.is_some());
                        if skinned && joints != Some(bones) {
                            return Err(format!(
                                "LOD{}: joints {joints:?} for {bones} bones",
                                lod.level
                            ));
                        }
                        if skinned && summary.skinned_meshes == 0 {
                            return Err(format!("LOD{}: no skinned mesh written", lod.level));
                        }
                    }
                    Ok(())
                });
            let _ignored = std::fs::remove_file(&file);
            result.err().map(|error| format!("{path}: {error}"))
        })
        .collect();

    assert!(failures.is_empty(), "failed: {failures:#?}");
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn animated_static_models_have_compact_bones_that_point_at_their_nodes() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let mut entries = sample(&archive, ".cga", ANIMATED_SAMPLE);
    entries.extend(sample(&archive, ".cgf", ANIMATED_SAMPLE));
    let tally = entries
        .par_iter()
        .map(|entry| {
            let mut tally = Tally::default();
            let path = entry.path();
            let source = match ModelSource::from_index(&index, path) {
                Ok(source) => source,
                Err(error) => {
                    tally.fail(path, format!("not read: {error}"));
                    return tally;
                }
            };
            let model = source.model();
            if model.bones.is_empty() {
                return tally;
            }
            tally.models = 1;
            tally.bones = model.bones.len() as u64;
            if let Some(bone) = model
                .bones
                .iter()
                .find(|bone| bone.node.is_some_and(|node| node >= model.nodes.len()))
            {
                tally.fail(path, format!("bone {} drives a missing node", bone.name));
            }
            check_bones(&model.bones, path, &mut tally);
            tally
        })
        .reduce(Tally::default, Tally::merge);

    println!("{} models with bones, {} bones", tally.models, tally.bones);
    assert!(tally.models > 0, "the sample holds models with bones");
    assert!(tally.failures.is_empty(), "failed: {:#?}", tally.failures);
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_vertices_of_every_skin_are_floats_so_the_quantisation_box_does_not_matter() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entries: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".skinm"))
        .collect();
    let (meshes, other): (u64, Vec<String>) = entries
        .par_iter()
        .map(|entry| {
            let mut bytes = Vec::new();
            let read = archive
                .open_entry(entry)
                .ok()
                .and_then(|reader| reader.take(MAX_FILE_LEN).read_to_end(&mut bytes).ok());
            let Some(file) = read.and_then(|_| ChunkFile::parse(bytes).ok()) else {
                return (0, vec![format!("{}: unreadable", entry.path())]);
            };
            let mut meshes = 0;
            let mut other = Vec::new();
            for (position, _) in file.chunks_of(ivo::SKIN_2) {
                let walked = file.chunk_data(position).and_then(mesh_streams);
                let Some(walked) = walked else {
                    other.push(format!("{}: no streams", entry.path()));
                    continue;
                };
                meshes += 1;
                let vertex = walked.streams.iter().find(|stream| stream.tag == VERTS_UVS);
                if vertex.is_none_or(|stream| stream.element_size != 20) {
                    other.push(format!("{}: {vertex:?}", entry.path()));
                }
            }
            (meshes, other)
        })
        .reduce(
            || (0, Vec::new()),
            |(left, mut errors), (right, more)| {
                errors.extend(more);
                (left + right, errors)
            },
        );

    println!(
        "{meshes} skin meshes, {} without 20-byte float vertices",
        other.len()
    );
    assert!(meshes > 0, "the Archive holds skin meshes");
    assert!(other.is_empty(), "quantised or unreadable: {other:#?}");
    Ok(())
}
