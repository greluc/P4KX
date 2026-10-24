// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, EntityInterior, InteriorOptions, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry, GlbOptions};
use p4kx_testkit::gltf::Node;
use p4kx_testkit::{ArchiveBuilder, ENTITY_INTERIOR_CLASS, check_glb, entity_interior_entries};

use super::pieces_of;

type Matrix = [[f32; 4]; 4];

fn multiply(parent: &Matrix, child: &Matrix) -> Matrix {
    std::array::from_fn(|column| {
        std::array::from_fn(|row| {
            (0..4)
                .map(|k| parent[k][row] * child[column][k])
                .sum::<f32>()
        })
    })
}

fn apply(matrix: &Matrix, point: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        matrix[3][row]
            + (0..3)
                .map(|column| matrix[column][row] * point[column])
                .sum::<f32>()
    })
}

fn collect(node: &Node<'_>, parent: &Matrix, out: &mut Vec<(String, Matrix)>) {
    let world = multiply(parent, &node.transform().matrix());
    out.push((node.name().unwrap_or_default().to_owned(), world));
    for child in node.children() {
        collect(&child, &world, out);
    }
}

fn y_up([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
}

fn sample() -> Assembly {
    let dir = tempfile::tempdir().expect("a folder");
    let path = dir.path().join("Data.p4k");
    entity_interior_entries()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)
        .expect("the Archive is written");
    let archive = Archive::open(&path).expect("the Archive opens");
    let entry = DataForgeEntry::read(&archive).expect("a DataForge");
    let forge = entry.open().expect("it opens");
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver
        .entities()
        .find(ENTITY_INTERIOR_CLASS)
        .expect("the Entity");
    let tree = resolver.resolve(&record).expect("its Loadout");
    let models = ArchiveModels::new(&archive);
    let hull = Assembly::build(&tree, &models, &AssemblyOptions::default()).expect("the hull");
    let interior = EntityInterior::build(&archive, &hull, &tree, &InteriorOptions::default())
        .expect("the Interior");
    hull.with_interior(interior)
}

#[test]
fn a_piece_of_the_interior_lies_where_the_glb_node_of_its_part_lies() {
    let assembly = sample();
    let mut bytes = Vec::new();
    assembly
        .write_glb(&GlbOptions::default(), &mut bytes)
        .expect("the GLB");
    let glb = check_glb(&bytes).expect("a valid GLB");
    let mut nodes = Vec::new();
    let identity: Matrix = std::array::from_fn(|c| std::array::from_fn(|r| f32::from(c == r)));
    for scene in glb.document.scenes() {
        for root in scene.nodes() {
            collect(&root, &identity, &mut nodes);
        }
    }

    let pieces = pieces_of(&assembly);
    let sizes: Vec<usize> = assembly
        .interior()
        .expect("an Interior")
        .containers()
        .iter()
        .map(|container| container.interior().parts().len())
        .collect();
    let mut compared = 0;
    let mut start = pieces.iter().filter(|piece| !piece.interior).count();
    for (at, size) in sizes.iter().enumerate().take(2) {
        let slice = &pieces[start..start + size];
        start += size;
        for piece in slice {
            let name = piece.name.trim_start_matches("Interior/");
            let same = |other: &&crate::assembly::pieces::Piece<'_>| {
                other.name.trim_start_matches("Interior/") == name
            };
            if piece.model.is_none() || slice.iter().filter(same).count() != 1 {
                continue;
            }
            let Some((_, matrix)) = nodes.iter().filter(|(node, _)| node == name).nth(at) else {
                continue;
            };
            compare(piece, matrix, name);
            compared += 1;
        }
    }
    assert!(compared >= 4, "{compared} parts with a node of their own");
    assert!(
        pieces
            .iter()
            .any(|piece| piece.interior && piece.world[1][3].abs() > 1.0),
        "a placement that moves the rooms away from the hull's origin"
    );
}

fn compare(piece: &crate::assembly::pieces::Piece<'_>, matrix: &Matrix, name: &str) {
    for probe in [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
        let viewer: [f32; 3] = std::array::from_fn(|row| {
            piece.world[row][3]
                + (0..3)
                    .map(|column| piece.world[row][column] * probe[column])
                    .sum::<f32>()
        });
        assert!(
            close(apply(matrix, y_up(probe)), y_up(viewer)),
            "{name} at {probe:?}: the file puts it at {:?}, the view at {:?}",
            apply(matrix, y_up(probe)),
            y_up(viewer)
        );
    }
}
