// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use super::offscreen_tests::open;
use super::{Colours, Frame, Look, Shading, Subject};
use crate::fixtures::{door_model, skin_model};
use crate::skeleton::Pose;
use crate::{Camera, Scene};

const SIZE: [u32; 2] = [96, 64];

fn frame(scene: &Arc<Scene>, pose: Pose) -> Frame {
    let mut camera = Camera::framing(scene.bounds());
    camera.fit(1.5);
    Frame {
        subject: Subject::Model(Arc::clone(scene)),
        uniforms: camera.uniforms(1.5),
        size: SIZE,
        look: Look {
            pose: Some(Arc::new(pose)),
            bones: None,
        },
        colours: Colours::default(),
        shading: Shading::Solid,
    }
}

fn scene_of(model: &p4kx_extraction::model::Model) -> Arc<Scene> {
    Arc::new(Scene::from_model(model).unwrap_or_else(|error| panic!("{error}")))
}

#[test]
fn the_report_tells_whether_the_skeleton_of_a_scene_is_on_the_device() {
    let Some(mut offscreen) = open("the_report_tells_whether_the_skeleton") else {
        return;
    };
    let rigged = scene_of(&skin_model());
    let mut bare = door_model();
    bare.bones.clear();
    let bare = scene_of(&bare);
    let report = offscreen.report();
    assert_eq!(report.skeleton_held(rigged.serial()), None, "not uploaded");

    assert!(offscreen.draw(&frame(&rigged, Pose::rest(2))).is_ok());
    assert_eq!(report.skeleton_held(rigged.serial()), Some(true), "held");

    assert!(offscreen.draw(&frame(&bare, Pose::rest(2))).is_ok());
    assert_eq!(report.skeleton_held(bare.serial()), Some(false), "none");
}

#[test]
fn a_pose_that_cannot_move_the_scene_does_not_draw_it_again() {
    let Some(mut offscreen) = open("a_pose_that_cannot_move_the_scene") else {
        return;
    };
    let mut bare = door_model();
    bare.bones.clear();
    let bare = scene_of(&bare);
    let rigged = scene_of(&skin_model());
    let report = offscreen.report();

    assert!(offscreen.draw(&frame(&bare, Pose::rest(2))).is_ok());
    let passes = report.passes();
    assert!(offscreen.draw(&frame(&bare, Pose::rest(2))).is_ok());
    assert_eq!(report.passes(), passes, "no skeleton, no new pose to draw");

    assert!(offscreen.draw(&frame(&rigged, Pose::rest(2))).is_ok());
    let passes = report.passes();
    assert!(offscreen.draw(&frame(&rigged, Pose::rest(2))).is_ok());
    assert_eq!(report.passes(), passes + 1, "a new pose of a rig is drawn");
}
