// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Animations of a real Archive: every `.caf` and `.dba` reads, the controller IDs are the CRC-32
//! of the bone names, and the first keys of a character animation sit near the skeleton's bind
//! pose; runs with `--ignored` when `P4KX_DATA_P4K` names one. No entry involved is encrypted, so
//! no Archive key is needed. The tables are printed (`--nocapture`).

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_archive::Archive;
use p4kx_extraction::model::{Animation, Dba};
use p4kx_extraction::{
    AnimationChoice, AnimationSet, ChunkSource, GlbOptions, ModelSource, PathIndex,
};
use p4kx_testkit::{check_glb, real_archive_path};
use rayon::prelude::*;

mod formats;
mod listing;
mod reach;
mod support;

use support::{Census, entries_ending_with, first_pose_distances};

type TestResult = Result<(), Box<dyn Error>>;

const SKELETON: &str = "Data\\Objects\\Characters\\Human\\male_v7\\export\\bhm_skeleton_v7.chr";
const IDLES: &str = "Data\\Animations\\Characters\\Human\\male_v7\\ai\\civ\\standing_idles\\";
const WEAPON_DBAS: &str = "Data\\Animations\\Characters\\Human\\male_v7\\weapons\\no_weapon\\";
const DOOR: &str = "Data\\Objects\\buildingsets\\human\\lowtech\\bravo\\pod\\pod_door_single_a.cga";
const DOOR_ANIMATION: &str = "pod_door_single_a_door_closed";
const GETUP: &str = "Data\\Animations\\Characters\\Human\\male_v7\\weapons\\no_weapon\\getup.dba";
const MIN_BOUND_SHARE: f64 = 0.85;
const MAX_MEDIAN_ANGLE: f32 = 0.3;
const MAX_MEDIAN_DISTANCE: f32 = 0.05;

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_caf_and_dba_of_the_archive_reads() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let cafs = entries_ending_with(&archive, ".caf");
    let dbas = entries_ending_with(&archive, ".dba");

    let caf_census = cafs
        .par_iter()
        .map(|path| {
            let mut census = Census::default();
            match ChunkSource::from_index(&index, path) {
                Ok(source) => match Animation::read_caf(path, source.file()) {
                    Ok(animation) => census.add(&animation),
                    Err(error) => census.fail(path, &error),
                },
                Err(error) => census.fail(path, &error),
            }
            census
        })
        .reduce(Census::default, Census::merge);
    let dba_census = dbas
        .par_iter()
        .map(|path| {
            let mut census = Census::default();
            match ChunkSource::from_index(&index, path) {
                Ok(source) => match Dba::read(path, source.file()) {
                    Ok(dba) => {
                        dba.animations
                            .iter()
                            .for_each(|animation| census.add(animation));
                        census.note_database(dba.animations.iter().map(Animation::key_count).sum());
                        dba.skipped
                            .iter()
                            .for_each(|skipped| census.fail(&skipped.path, &skipped.reason));
                    }
                    Err(error) => census.fail(path, &error),
                },
                Err(error) => census.fail(path, &error),
            }
            census
        })
        .reduce(Census::default, Census::merge);

    println!("{} .caf files: {}", cafs.len(), caf_census.summary());
    println!("{} .dba files: {}", dbas.len(), dba_census.summary());
    assert!(
        !cafs.is_empty() && !dbas.is_empty(),
        "the Archive holds animations"
    );
    assert!(
        caf_census.failures.is_empty() && dba_census.failures.is_empty(),
        "failed: {:#?} {:#?}",
        caf_census.failures.iter().take(20).collect::<Vec<_>>(),
        dba_census.failures.iter().take(20).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn controller_ids_are_the_crc32_of_the_bone_names_as_spelled() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let mut paths = entries_ending_with(&archive, ".chr");
    paths.extend(entries_ending_with(&archive, ".cga").into_iter().take(2000));
    let (bones, mismatches): (u64, Vec<String>) = paths
        .par_iter()
        .map(|path| support::controller_mismatches(&index, path))
        .reduce(
            || (0, Vec::new()),
            |(bones, mut all), (more, found)| {
                all.extend(found);
                (bones + more, all)
            },
        );
    println!(
        "{bones} bones of {} files; {} controller IDs are not the CRC-32 of the name",
        paths.len(),
        mismatches.len()
    );
    assert!(bones > 0, "the sample holds bones");
    assert!(
        mismatches.is_empty(),
        "{:#?}",
        mismatches.iter().take(20).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_first_keys_of_a_character_animation_sit_near_the_bind_pose() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let skeleton = ModelSource::from_index(&index, SKELETON)?;
    let bones = &skeleton.model().bones;

    let mut animations: Vec<Animation> = Vec::new();
    for path in entries_ending_with(&archive, ".caf")
        .into_iter()
        .filter(|path| path.starts_with(IDLES))
        .take(40)
    {
        animations.push(Animation::read_caf(
            &path,
            ChunkSource::from_index(&index, &path)?.file(),
        )?);
    }
    for path in entries_ending_with(&archive, ".dba")
        .into_iter()
        .filter(|path| path.starts_with(WEAPON_DBAS))
    {
        animations
            .extend(Dba::read(&path, ChunkSource::from_index(&index, &path)?.file())?.animations);
    }

    let pose = first_pose_distances(&animations, bones);
    println!(
        "{} animations, {} tracks, {:.3} bound; median first-key distance to the bind pose: \
         {:.3} rad, {:.4} m",
        animations.len(),
        pose.tracks,
        pose.bound_share(),
        pose.median_angle,
        pose.median_distance
    );
    assert!(pose.tracks > 0, "the sample has tracks");
    assert!(
        pose.bound_share() >= MIN_BOUND_SHARE,
        "bound {:.3}",
        pose.bound_share()
    );
    assert!(
        pose.median_angle <= MAX_MEDIAN_ANGLE,
        "rotations: {:.3}",
        pose.median_angle
    );
    assert!(
        pose.median_distance <= MAX_MEDIAN_DISTANCE,
        "translations: {:.4}",
        pose.median_distance
    );
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn keyframe_offsets_of_databases_reach_past_their_chunk_span() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let dbas = entries_ending_with(&archive, ".dba");

    let reaches: BTreeMap<String, reach::Reach> = dbas
        .par_iter()
        .filter_map(|path| {
            let source = ChunkSource::from_index(&index, path).ok()?;
            Some((path.clone(), reach::of(source.file())?))
        })
        .collect();

    let past = reaches
        .values()
        .filter(|reach| reach.beyond_span > 0)
        .count();
    let furthest = reaches
        .values()
        .map(|reach| reach.beyond_span)
        .max()
        .unwrap_or(0);
    let beyond_file = reaches
        .values()
        .filter(|reach| reach.beyond_file > 0)
        .count();
    println!(
        "{} databases measured, {past} with data starting past the DbaData span (at most {furthest} \
         bytes), {beyond_file} with data starting past the end of the file",
        reaches.len()
    );
    assert!(!reaches.is_empty(), "the Archive holds databases");
    assert_eq!(beyond_file, 0, "no key data starts outside the file");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_character_skeleton_exports_with_the_animations_of_a_database() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let skeleton = ModelSource::from_index(&index, SKELETON)?;
    let choice = AnimationChoice::Named(vec![GETUP.to_owned()]);
    let animations = AnimationSet::from_archive(&index, SKELETON, &choice, &[])?;
    assert!(
        animations.animations.len() >= 4,
        "the database holds several animations"
    );

    let out = tempfile::tempdir()?;
    let file = out.path().join("skeleton.glb");
    let summary = skeleton.write_glb_animated(&animations, &GlbOptions::default(), &file)?;

    let glb = check_glb(&std::fs::read(&file)?)?;
    assert_eq!(
        glb.document.animations().len(),
        animations.animations.len(),
        "all written"
    );
    assert_eq!(
        summary.bones,
        skeleton.model().bones.len(),
        "bones as nodes"
    );
    for written in &summary.animations {
        assert!(written.channels > 0, "{} moves something", written.name);
        assert!(
            written.duration_seconds > 0.0,
            "{} has a duration",
            written.name
        );
        assert!(
            written.skipped_tracks.len() * 10 <= written.channels,
            "{} skipped {} tracks for {} channels",
            written.name,
            written.skipped_tracks.len(),
            written.channels
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_static_model_exports_with_the_animation_found_at_the_mirrored_path() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let door = ModelSource::from_index(&index, DOOR)?;
    let choice = AnimationChoice::Named(vec![DOOR_ANIMATION.to_owned()]);
    let animations = AnimationSet::from_archive(&index, DOOR, &choice, &[])?;
    assert_eq!(
        animations.animations.len(),
        1,
        "the door's caf under Data\\Animations"
    );

    let out = tempfile::tempdir()?;
    let file = out.path().join("door.glb");
    let summary = door.write_glb_animated(&animations, &GlbOptions::default(), &file)?;

    let glb = check_glb(&std::fs::read(&file)?)?;
    let channel = glb
        .document
        .animations()
        .next()
        .and_then(|animation| animation.channels().next())
        .ok_or("a channel")?;
    let target = glb
        .document
        .nodes()
        .nth(channel.target().node().index())
        .and_then(|node| node.name());
    assert_eq!(
        target,
        Some("pod_door_single_a_mesh"),
        "the door's mesh node"
    );
    assert_eq!(summary.bones, 0, "helper bones are not written");
    assert!(
        summary
            .animations
            .iter()
            .all(|animation| animation.skipped_tracks.is_empty()),
        "every track found its bone"
    );
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn only_the_key_formats_the_reader_knows_are_in_use() -> TestResult {
    const ROTATIONS: [u8; 3] = [0x80, 0x82, 0x83];
    const POSITIONS: [u8; 3] = [0xC0, 0xC1, 0xC2];
    const TIMES: [u8; 3] = [0x40, 0x41, 0x42];
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let paths: Vec<String> = entries_ending_with(&archive, ".caf")
        .into_iter()
        .chain(entries_ending_with(&archive, ".dba"))
        .collect();

    let formats = paths
        .par_iter()
        .filter_map(|path| ChunkSource::from_index(&index, path).ok())
        .map(|source| formats::of(source.file()))
        .reduce(formats::Formats::default, formats::Formats::merge);

    println!(
        "controllers with rotation codes {}, position codes {}, time codes {}; chunk versions {}",
        formats::list(&formats.rotations),
        formats::list(&formats.positions),
        formats::list(&formats.times),
        formats::versions(&formats.chunk_versions)
    );
    assert!(
        formats
            .rotations
            .keys()
            .all(|code| ROTATIONS.contains(code)),
        "rotation codes"
    );
    assert!(
        formats
            .positions
            .keys()
            .all(|code| POSITIONS.contains(code)),
        "position codes"
    );
    assert!(
        formats.times.keys().all(|code| TIMES.contains(code)),
        "time codes"
    );
    Ok(())
}
