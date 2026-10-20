//! The GLB of the synthetic ship: placed nodes, transforms, materials and the level of detail.

mod assembly_support;

use assembly_support::{TestResult, assemble, entries, fixture};
use p4kx_extraction::Archive;
use p4kx_gltf::{GlbOptions, GlbSummary};
use p4kx_testkit::gltf::{Node, Semantic};
use p4kx_testkit::{CheckedGlb, check_glb};

fn export(lod: u32) -> Result<(CheckedGlb, GlbSummary), Box<dyn std::error::Error>> {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Ship", lod)?;
    let mut bytes = Vec::new();
    let summary = assembly.write_glb(&GlbOptions::default().with_lod(lod), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

fn node<'a>(glb: &'a CheckedGlb, name: &str) -> Result<Node<'a>, Box<dyn std::error::Error>> {
    glb.document
        .nodes()
        .find(|node| node.name() == Some(name))
        .ok_or_else(|| format!("no node {name}").into())
}

fn close(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 1e-5)
}

#[test]
fn the_glb_holds_a_node_per_part_named_after_port_and_entity_in_the_loadout_hierarchy() -> TestResult
{
    let (glb, summary) = export(0)?;

    let ship = node(&glb, "TEST_Ship")?;
    let mount = node(&glb, "hardpoint_weapon_nose/TEST_Mount")?;
    let gun = node(&glb, "hardpoint_class_2/TEST_Gun")?;
    let children =
        |parent: &Node<'_>| -> Vec<usize> { parent.children().map(|c| c.index()).collect() };
    assert!(
        children(&ship).contains(&mount.index()),
        "the mount hangs below the ship"
    );
    assert!(
        children(&mount).contains(&gun.index()),
        "the gun hangs below the mount"
    );
    for name in [
        "hardpoint_relay/TEST_Relay",
        "hardpoint_seat/TEST_Seat",
        "hardpoint_door/TEST_Door",
    ] {
        assert!(
            children(&ship).contains(&node(&glb, name)?.index()),
            "{name} hangs below the ship"
        );
    }
    assert_eq!(summary.parts, 4, "ship, mount, gun and relay have models");
    assert_eq!(summary.meshes, 4, "one mesh node each");
    assert!(
        node(&glb, "hardpoint_seat/TEST_Seat")?.mesh().is_none(),
        "no geometry, no mesh"
    );
    Ok(())
}

#[test]
fn part_nodes_carry_the_transforms_of_their_ports_in_y_up_axes() -> TestResult {
    let (glb, _) = export(0)?;

    let mount = node(&glb, "hardpoint_weapon_nose/TEST_Mount")?;
    let (translation, rotation, scale) = mount.transform().decomposed();
    assert!(
        close(&translation, &[0.0, 1.0, -4.0]),
        "(0, 4, 1) in Z up: {translation:?}"
    );
    let half = std::f32::consts::FRAC_1_SQRT_2;
    assert!(
        close(&rotation, &[0.0, half, 0.0, half]),
        "90 degrees about up: {rotation:?}"
    );
    assert!(close(&scale, &[1.0; 3]), "{scale:?}");
    let gun = node(&glb, "hardpoint_class_2/TEST_Gun")?;
    assert!(
        close(&gun.transform().decomposed().0, &[0.0, 0.5, 0.0]),
        "the gun's helper is 0.5 m above the mount's origin"
    );
    let relay = node(&glb, "hardpoint_relay/TEST_Relay")?;
    assert!(
        close(&relay.transform().decomposed().0, &[3.0, 0.0, 0.0]),
        "the node named like the port"
    );
    Ok(())
}

#[test]
fn equal_materials_are_written_once_and_the_entitys_library_is_used() -> TestResult {
    let (glb, summary) = export(0)?;

    assert_eq!(
        summary.materials, 4,
        "red and green, two sub-materials each; the mount shares the ship's red"
    );
    assert_eq!(glb.document.textures().len(), 0, "plain colours");
    let gun = glb
        .document
        .meshes()
        .find(|mesh| mesh.name() == Some("cube_mesh"))
        .ok_or("a mesh")?;
    let colours: Vec<[f32; 4]> = glb
        .document
        .materials()
        .map(|material| material.pbr_metallic_roughness().base_color_factor())
        .collect();
    assert!(
        colours.iter().any(|c| close(&c[..3], &[0.1, 0.8, 0.1])),
        "green: {colours:?}"
    );
    assert!(
        colours.iter().any(|c| close(&c[..3], &[0.8, 0.1, 0.1])),
        "red: {colours:?}"
    );
    assert!(
        !colours.iter().any(|c| close(&c[..3], &[0.1, 0.1, 0.8])),
        "the model's own blue library is replaced: {colours:?}"
    );
    assert!(gun.primitives().len() > 0);
    Ok(())
}

#[test]
fn the_level_of_detail_applies_to_the_whole_assembly() -> TestResult {
    let extent = |glb: &CheckedGlb| -> Result<f64, Box<dyn std::error::Error>> {
        let mesh = glb
            .document
            .meshes()
            .find(|mesh| mesh.name() == Some("ship_mesh"))
            .ok_or("the ship mesh")?;
        let primitive = mesh.primitives().next().ok_or("a primitive")?;
        let max = primitive
            .get(&Semantic::Positions)
            .and_then(|accessor| accessor.max())
            .ok_or("bounds")?;
        Ok(max
            .as_array()
            .and_then(|values| values.first())
            .and_then(p4kx_testkit::gltf::json::Value::as_f64)
            .ok_or("x")?)
    };
    let (first, summary) = export(0)?;
    let (second, lower) = export(1)?;

    assert!((extent(&first)? - 2.0).abs() < 1e-5, "the cube is 4 m wide");
    assert!(
        (extent(&second)? - 1.0).abs() < 1e-5,
        "LOD1 is half the size"
    );
    assert_eq!(summary.lod_fallbacks, 0);
    assert_eq!(
        lower.lod_fallbacks, 3,
        "the mount, the gun and the relay's gun have no LOD1"
    );
    Ok(())
}
