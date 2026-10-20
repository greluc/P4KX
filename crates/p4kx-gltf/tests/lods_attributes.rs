//! Levels of detail, tangents, vertex colours and sub-object hierarchies written as GLB.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, GlbSummary, write_glb};
use p4kx_model::{CompanionError, CompanionPath, LodSelection, Model};
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{
    CUBE_CAPS_MATERIAL, CUBE_SIDES_MATERIAL, CheckedGlb, TestModel, TestNode, TestNormals,
    TestSubmesh, check_glb,
};

type TestResult = Result<(), Box<dyn Error>>;

fn caps_only() -> TestModel {
    TestModel::cube().with_submeshes(vec![TestSubmesh {
        material_id: CUBE_CAPS_MATERIAL,
        node: 1,
        first_index: 0,
        index_count: 12,
        first_vertex: 0,
        vertex_count: 8,
    }])
}

fn model(cube: &TestModel) -> Result<Model, Box<dyn Error>> {
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        let CompanionPath::Sibling(name) = path else {
            return Ok(None);
        };
        Ok(match name {
            "cube.cgfm" => Some(cube.cgfm()),
            "cube_lod1.cgfm" => cube.lod_cgfm(1),
            _ => None,
        })
    };
    Ok(Model::read_with(
        "cube.cgf",
        &ChunkFile::parse(cube.cgf())?,
        &mut companions,
        LodSelection::All,
    )?)
}

fn glb_with(
    cube: &TestModel,
    options: GlbOptions,
) -> Result<(CheckedGlb, GlbSummary), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let summary = write_glb(&model(cube)?, &options, &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

fn first_primitive(glb: &CheckedGlb) -> Result<p4kx_testkit::gltf::Primitive<'_>, Box<dyn Error>> {
    Ok(glb
        .document
        .meshes()
        .next()
        .and_then(|mesh| mesh.primitives().next())
        .ok_or("a primitive")?)
}

#[test]
fn the_chosen_level_of_detail_is_exported_with_the_nodes_of_the_model() -> TestResult {
    let cube = TestModel::cube().with_lod(caps_only());

    let (lod0, summary0) = glb_with(&cube, GlbOptions::default())?;
    assert_eq!(
        (summary0.vertices, summary0.triangles, summary0.primitives),
        (24, 12, 2),
        "LOD0 by default"
    );
    assert_eq!(lod0.document.nodes().len(), 2, "both nodes");

    let (lod1, summary1) = glb_with(&cube, GlbOptions::default().with_lod(1))?;
    assert_eq!(
        (
            summary1.vertices,
            summary1.triangles,
            summary1.primitives,
            summary1.materials
        ),
        (8, 4, 1, 1),
        "LOD1 is the caps only"
    );
    assert_eq!(lod1.document.nodes().len(), 2, "the nodes are the same");
    let names: Vec<Option<&str>> = lod1.document.nodes().map(|node| node.name()).collect();
    assert_eq!(names, [Some("cube"), Some("cube_mesh")], "same hierarchy");
    let mesh = lod1
        .document
        .nodes()
        .nth(1)
        .and_then(|node| node.mesh())
        .ok_or("the mesh on cube_mesh")?;
    assert_eq!(mesh.primitives().len(), 1, "one primitive");
    Ok(())
}

#[test]
fn an_absent_level_of_detail_is_an_error() -> TestResult {
    let cube = TestModel::cube().with_lod(caps_only());
    let options = GlbOptions::default().with_lod(2);
    let error = write_glb(&model(&cube)?, &options, &mut Vec::new())
        .err()
        .ok_or("an error")?;
    assert_eq!(
        error.to_string(),
        "the model has no LOD2; it has LOD0, LOD1"
    );
    Ok(())
}

#[test]
fn tangents_and_colours_are_written_when_the_mesh_has_them() -> TestResult {
    let colours: Vec<[u8; 4]> = (0..24_u8).map(|at| [at, 100, 200, 255]).collect();
    let cube = TestModel::cube()
        .with_normals(TestNormals::Compressed)
        .with_mirrored_tangents()
        .with_colours(colours.clone());
    let (glb, _) = glb_with(&cube, GlbOptions::default())?;
    let primitive = first_primitive(&glb)?;

    let tangents = glb.floats(&primitive.get(&Semantic::Tangents).ok_or("TANGENT")?);
    let expected = cube.tangents();
    assert_eq!(tangents.len(), expected.len() * 4, "four floats per vertex");
    for (found, wanted) in tangents.as_chunks::<4>().0.iter().zip(&expected) {
        let [x, y, z, w] = *wanted;
        let converted = [x, z, -y, w];
        for (a, b) in found.iter().zip(converted) {
            assert!((a - b).abs() < 1e-3, "{found:?} is not {converted:?}");
        }
    }

    let accessor = primitive.get(&Semantic::Colors(0)).ok_or("COLOR_0")?;
    assert!(accessor.normalized(), "colours are normalised bytes");
    assert_eq!(accessor.count(), 24, "one colour per vertex");
    let found = glb.bytes(&accessor);
    let wanted: Vec<u8> = colours.iter().flatten().copied().collect();
    assert_eq!(found, wanted, "red, green, blue and alpha per vertex");
    Ok(())
}

#[test]
fn colours_are_left_out_when_all_white_or_not_wanted() -> TestResult {
    let (glb, _) = glb_with(&TestModel::cube(), GlbOptions::default())?;
    let primitive = first_primitive(&glb)?;
    assert!(primitive.get(&Semantic::Colors(0)).is_none(), "all white");
    assert!(
        primitive.get(&Semantic::Tangents).is_none(),
        "no tangent frame"
    );

    let coloured = TestModel::cube().with_colours(vec![[1, 2, 3, 255]; 24]);
    let (with, _) = glb_with(&coloured, GlbOptions::default())?;
    assert!(
        first_primitive(&with)?.get(&Semantic::Colors(0)).is_some(),
        "colours are written by default"
    );
    let (without, _) = glb_with(&coloured, GlbOptions::default().without_vertex_colours())?;
    assert!(
        first_primitive(&without)?
            .get(&Semantic::Colors(0))
            .is_none(),
        "unless they are switched off"
    );
    Ok(())
}

fn node(name: &str, parent: Option<u16>, local: [[f32; 4]; 3]) -> TestNode {
    TestNode {
        name: name.to_owned(),
        parent,
        local,
        kind: 0,
    }
}

fn chain_of_sub_objects() -> TestModel {
    let nodes = vec![
        node(
            "ship",
            None,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        ),
        node(
            "arm",
            Some(0),
            [
                [0.0, -1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        ),
        node(
            "hand",
            Some(1),
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 2.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        ),
    ];
    let submeshes = vec![
        TestSubmesh {
            material_id: CUBE_CAPS_MATERIAL,
            node: 1,
            first_index: 0,
            index_count: 12,
            first_vertex: 0,
            vertex_count: 8,
        },
        TestSubmesh {
            material_id: CUBE_SIDES_MATERIAL,
            node: 2,
            first_index: 12,
            index_count: 24,
            first_vertex: 8,
            vertex_count: 16,
        },
    ];
    TestModel::cube()
        .with_nodes(nodes)
        .with_submeshes(submeshes)
}

#[test]
fn sub_objects_keep_their_hierarchy_and_transforms() -> TestResult {
    let cube = chain_of_sub_objects();
    let (glb, summary) = glb_with(&cube, GlbOptions::default())?;

    let hierarchy: Vec<(Option<&str>, Vec<usize>, bool)> = glb
        .document
        .nodes()
        .map(|node| {
            (
                node.name(),
                node.children().map(|child| child.index()).collect(),
                node.mesh().is_some(),
            )
        })
        .collect();
    assert_eq!(
        hierarchy,
        [
            (Some("ship"), vec![1], false),
            (Some("arm"), vec![2], true),
            (Some("hand"), vec![], true),
        ],
        "ship holds arm holds hand, and the parts that have geometry have meshes"
    );
    assert_eq!(summary.meshes, 2, "one mesh per sub-object");

    let arm = glb.document.nodes().nth(1).ok_or("arm")?;
    let (translation, rotation, _) = arm.transform().decomposed();
    assert_eq!(translation, [1.0, 0.0, 0.0], "X stays X");
    let half = std::f32::consts::FRAC_1_SQRT_2;
    for (found, wanted) in rotation.iter().zip([0.0, half, 0.0, half]) {
        assert!(
            (found - wanted).abs() < 1e-4,
            "a quarter turn about Z is one about Y: {rotation:?}"
        );
    }
    let hand = glb.document.nodes().nth(2).ok_or("hand")?;
    let (translation, _, _) = hand.transform().decomposed();
    assert_eq!(translation, [0.0, 0.0, -2.0], "Y becomes −Z");
    Ok(())
}
