// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use super::{
    AssemblyScene, DrawList, Group, Item, Left, MIN_PART_PIXELS, PartDraw, PartInfo, hidden_mask,
};
use crate::scene::{Bounds, Rows};
use crate::{Camera, Matrix};

const ASPECT: f32 = 1.5;
const HEIGHT: f32 = 600.0;

fn at(x: f32, y: f32, z: f32) -> Rows {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

fn info(path: &str, parent: Option<usize>) -> PartInfo {
    PartInfo {
        name: path.to_owned(),
        path: path.to_owned(),
        parent,
        triangles: 12,
    }
}

fn draw(part: usize, world: Rows, radius: f32, items: std::ops::Range<u32>) -> PartDraw {
    PartDraw {
        part,
        centre: [world[0][3], world[1][3], world[2][3]],
        world,
        radius,
        items,
    }
}

fn group(translucent: bool) -> Group {
    Group {
        first_index: 0,
        index_count: 36,
        texture: None,
        translucent,
    }
}

fn scene() -> AssemblyScene {
    let item = |group| Item {
        group,
        colour: [0.5, 0.5, 0.5, 1.0],
    };
    AssemblyScene {
        vertices: Vec::new(),
        indices: vec![0; 36],
        groups: vec![group(false), group(true)],
        items: vec![item(0), item(0), item(0), item(1), item(0)],
        draws: vec![
            draw(0, at(0.0, 0.0, 0.0), 1.0, 0..1),
            draw(1, at(2.0, 0.0, 0.0), 1.0, 1..2),
            draw(2, at(0.0, 2.0, 0.0), 1.0, 2..4),
            draw(3, at(0.0, 0.0, 2.0), 1.0, 4..5),
        ],
        parts: vec![
            info("", None),
            info("a", Some(0)),
            info("a/b", Some(1)),
            info("c", Some(0)),
        ],
        textures: Vec::new(),
        meshes: 1,
        bounds: Bounds {
            min: [-1.0; 3],
            max: [3.0; 3],
        },
        left: Left::default(),
        serial: 1,
    }
}

fn view(scene: &AssemblyScene) -> Matrix {
    let mut camera = Camera::framing(scene.bounds());
    camera.fit(ASPECT);
    camera.uniforms(ASPECT).view_proj
}

#[test]
fn parts_sharing_a_mesh_range_become_one_run_of_instances() {
    let scene = scene();

    let list = DrawList::of(&scene, &[false; 4], &view(&scene), HEIGHT);

    assert_eq!(list.visible_parts(), 4, "every part is in view");
    assert_eq!(list.instances().len(), 5, "one instance per item");
    let runs: Vec<(u32, u32)> = list
        .draws()
        .iter()
        .map(|run| (run.group, run.count))
        .collect();
    assert_eq!(runs, vec![(0, 4), (1, 1)], "four share the opaque group");
    assert_eq!(list.draws().first().map(|run| run.first), Some(0), "start");
    assert_eq!(list.draws().get(1).map(|run| run.first), Some(4), "next");
}

#[test]
fn a_hidden_branch_hides_its_parts_and_their_descendants() {
    let scene = scene();
    let hidden: HashSet<String> = ["a".to_owned()].into();

    let mask = hidden_mask(scene.parts(), &hidden);
    let list = DrawList::of(&scene, &mask, &view(&scene), HEIGHT);

    assert_eq!(mask, vec![false, true, true, false], "a and a/b");
    assert_eq!(list.visible_parts(), 2, "the root and c remain");
    assert_eq!(list.hidden_parts(), 2, "two were hidden");
    assert_eq!(list.instances().len(), 2, "their instances are gone");
}

#[test]
fn the_root_is_never_hidden_by_an_empty_path() {
    let scene = scene();
    let hidden: HashSet<String> = [String::new()].into();

    assert_eq!(
        hidden_mask(scene.parts(), &hidden),
        vec![false; 4],
        "an empty path names no port"
    );
}

#[test]
fn a_part_outside_the_view_is_culled() {
    let mut scene = scene();
    scene.draws.push(draw(3, at(1.0e6, 0.0, 0.0), 1.0, 4..5));
    let matrix = view(&scene);

    let list = DrawList::of(&scene, &[false; 4], &matrix, HEIGHT);

    assert_eq!(list.outside_parts(), 1, "the far part");
    assert_eq!(list.visible_parts(), 4, "the others");
}

#[test]
fn a_part_smaller_than_a_pixel_is_not_drawn() {
    let mut scene = scene();
    scene.draws.push(draw(3, at(1.0, 1.0, 1.0), 1.0e-4, 4..5));
    let matrix = view(&scene);

    let list = DrawList::of(&scene, &[false; 4], &matrix, HEIGHT);

    assert_eq!(list.small_parts(), 1, "below {MIN_PART_PIXELS} px");
    assert_eq!(list.instances().len(), 5, "only the visible items");
}

#[test]
fn the_work_of_a_frame_follows_the_parts_not_the_triangles() {
    let mut scene = scene();
    scene.indices = vec![0; 3 * 5_000_000];
    scene.draws.clear();
    scene.parts = std::iter::once(info("", None))
        .chain((0..4000).map(|_| info("p", Some(0))))
        .collect();
    for part in 0..4000_u16 {
        scene.draws.push(draw(
            usize::from(part) + 1,
            at(f32::from(part) * 0.001, 0.0, 0.0),
            1.0,
            0..1,
        ));
    }
    let matrix = view(&scene);

    let list = DrawList::of(&scene, &vec![false; 4001], &matrix, HEIGHT);

    assert_eq!(list.draws().len(), 1, "one draw call for 4000 instances");
    assert_eq!(list.instances().len(), 4000, "one instance each");
}
