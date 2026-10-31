// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The expression targets of what follows the head, at every level of detail.

use std::sync::Arc;

use p4kx_gltf::MorphTarget;
use p4kx_model::Model;

use super::follow_morphs;
use crate::armour::head_move::{HeadMove, followed, level_meshes};
use crate::armour::test_models::{TestResult, hair, head_with_jaw};

fn blink() -> MorphTarget {
    MorphTarget {
        name: "blink".to_owned(),
        vertices: vec![0],
        deltas: vec![[0.0, 0.0, 0.002]],
    }
}

/// The hair as it is read at `level`: its first level of detail is that level.
fn hair_at(level: usize) -> TestResult<Model> {
    let mut hair = hair()?;
    hair.lods.drain(..level);
    Ok(hair)
}

fn follow(model: &Model, head: &Model, targets: &[MorphTarget]) -> TestResult<super::Followed> {
    let mesh = head
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .ok_or("a head mesh")?;
    Ok(follow_morphs(model, mesh, targets))
}

#[test]
fn the_hair_at_every_level_gets_the_targets_the_head_has_there() -> TestResult {
    let head = head_with_jaw()?;

    for level in 0..2 {
        let hair = hair_at(level)?;
        assert_eq!(
            hair.lods.first().map(|lod| usize::try_from(lod.level)),
            Some(Ok(level))
        );

        let made = follow(&hair, &head, &[blink()])?;

        assert_eq!(made.of, [0], "level {level}");
        let [target] = made.targets.as_slice() else {
            return Err(format!("one target at level {level}").into());
        };
        assert_eq!(target.name, "blink");
        assert!(
            target.deltas.iter().all(|by| by[2] > 0.0),
            "level {level}: {:?}",
            target.deltas
        );
    }
    Ok(())
}

#[test]
fn what_follows_the_head_has_one_target_per_head_target_at_a_lower_level() -> TestResult {
    let head = Arc::new(head_with_jaw()?);
    let hair = Arc::new(hair_at(1)?);
    let moved = HeadMove {
        before: Arc::clone(&head),
        after: Arc::clone(&head),
        morphs: vec![blink()],
    };

    let worn = followed(&hair, &moved);

    assert_eq!(worn.morphs.len(), worn.follows.len());
    assert_eq!(worn.follows, [0]);
    assert_eq!(level_meshes(&worn.model), 1);
    Ok(())
}

#[test]
fn a_head_without_targets_gives_no_targets_at_a_lower_level() -> TestResult {
    let head = head_with_jaw()?;

    let made = follow(&hair_at(1)?, &head, &[])?;

    assert!(made.targets.is_empty() && made.of.is_empty());
    Ok(())
}
