// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_testkit::{TestContainer, TestEntry, TestMtl, TestMtlMaterial};
use serde_json::json;

use crate::support::{Install, TestResult};

const LIBRARY: &str = "Data/Objects/Test/hull.mtl";

fn library() -> TestMtl {
    TestMtl::multi(&[
        TestMtlMaterial::new("plate")
            .shader("Illum")
            .surface_type("metal_dense")
            .diffuse([0.5, 0.25, 0.125])
            .opacity(0.75)
            .texture("TexSlot1", "objects/test/plate_diff.dds")
            .texture("TexSlot2", "objects/test/plate_ddna.dds")
            .public_param("WearBlendBase", "0.5")
            .layer("paint", "1"),
        TestMtlMaterial::new("glass").shader("GlassPBR").glow(2.0),
        TestMtlMaterial::new("hidden").flags(0x0400),
    ])
    .with_default_palette("libs/foundry/records/tintpalette/test.xml")
}

fn install() -> TestResult<Install> {
    let pack = TestContainer::new().entry(TestEntry::stored(
        "mats\\inner.mtl",
        TestMtl::single(&TestMtlMaterial::new("solo")).text_xml(),
    ));
    Install::new(vec![
        TestEntry::zstd(LIBRARY, library().cryxml()),
        TestEntry::stored("Data/Objects/Test/text.mtl", library().text_xml()),
        TestEntry::stored("Data/Objects/Test/pack.socpak", pack.build()),
        TestEntry::stored("Data/Objects/Test/bad.mtl", "<Other/>"),
    ])
}

#[test]
fn a_library_lists_its_sub_materials_with_shader_textures_and_parameters() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let summary = server.ok(
        "mtl_summary",
        &json!({ "path": "data\\objects\\test\\HULL.mtl" }),
    )?;

    assert_eq!(summary["path"], LIBRARY, "the Entry path");
    assert_eq!(summary["single"], false, "a list");
    assert_eq!(summary["total"], 3, "three sub-materials");
    assert_eq!(
        summary["default_palette"], "libs/foundry/records/tintpalette/test.xml",
        "the default palette"
    );
    let plate = &summary["materials"][0];
    assert_eq!(plate["id"], 0, "the material ID");
    assert_eq!(plate["name"], "plate", "the name");
    assert_eq!(plate["shader"], "Illum", "the shader");
    assert_eq!(plate["surface_type"], "metal_dense", "the surface");
    assert_eq!(plate["opacity"], 0.75, "the opacity");
    assert_eq!(
        plate["diffuse"],
        json!([0.5, 0.25, 0.125]),
        "the diffuse colour"
    );
    assert_eq!(
        plate["textures"][1],
        json!({ "map": "TexSlot2", "file": "objects/test/plate_ddna.dds" }),
        "the texture slots"
    );
    assert_eq!(
        plate["public_params"][0]["name"], "WearBlendBase",
        "the parameter"
    );
    assert_eq!(plate["layers"][0]["name"], "paint", "the layer");
    assert_eq!(summary["materials"][1]["glow"], 2.0, "glow");
    assert_eq!(summary["materials"][2]["no_draw"], true, "the no-draw flag");
    Ok(())
}

#[test]
fn text_xml_reads_like_cryxml_and_a_page_continues_with_its_cursor() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let binary = server.ok("mtl_summary", &json!({ "path": LIBRARY }))?;

    let first = server.ok(
        "mtl_summary",
        &json!({ "path": "Data/Objects/Test/text.mtl", "limit": 2 }),
    )?;
    let cursor = first["next_cursor"].as_str().ok_or("a next page")?;
    let second = server.ok(
        "mtl_summary",
        &json!({ "path": "Data/Objects/Test/text.mtl", "limit": 2, "cursor": cursor }),
    )?;

    assert_eq!(
        first["materials"].as_array().map(Vec::len),
        Some(2),
        "a page of two"
    );
    assert_eq!(second["materials"][0]["id"], 2, "the next page continues");
    assert_eq!(second["next_cursor"], json!(null), "and ends");
    assert_eq!(
        first["materials"][0], binary["materials"][0],
        "the same material from both forms"
    );
    Ok(())
}

#[test]
fn a_library_inside_a_container_and_bad_files_are_handled() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let inner = server.ok(
        "mtl_summary",
        &json!({ "path": "Data/Objects/Test/pack.socpak::mats/inner.mtl" }),
    )?;
    let bad = server.error(
        "mtl_summary",
        &json!({ "path": "Data/Objects/Test/bad.mtl" }),
    )?;
    let unknown = server.error("mtl_summary", &json!({ "path": "Data/none.mtl" }))?;

    assert_eq!(inner["single"], true, "one material");
    assert_eq!(inner["materials"][0]["name"], "solo", "its name");
    assert_eq!(
        inner["path"], "Data/Objects/Test/pack.socpak::mats\\inner.mtl",
        "addressed with its container"
    );
    assert!(bad.contains("cannot read the material library"), "{bad}");
    assert!(unknown.contains("holds no Entry"), "{unknown}");
    Ok(())
}
