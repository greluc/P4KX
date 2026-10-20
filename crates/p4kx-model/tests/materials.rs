//! Reading `.mtl` files into material libraries through the public API.

use std::error::Error;

use p4kx_model::{MAX_SUB_MATERIALS, MAX_TEXTURE_SLOTS, MaterialError, MaterialLibrary};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement, TestMtl, TestMtlMaterial};
use proptest::prelude::*;
type TestResult = Result<(), Box<dyn Error>>;

fn hull() -> TestMtlMaterial {
    TestMtlMaterial::new("hull")
        .shader("HardSurface")
        .surface_type("metal_dense")
        .string_gen_mask("%NORMAL_MAP%VERTCOLORS")
        .diffuse([0.25, 0.5, 0.75])
        .specular([1.0, 0.9, 0.8])
        .emissive([0.1, 0.2, 0.3])
        .glow(2.0)
        .opacity(0.5)
        .shininess(204.0)
        .alpha_test(0.4)
        .flags(0x0000_0002)
        .texture("TexSlot1", "Objects/Test/hull_diff.tif")
        .texture("TexSlot2", "Objects/Test/hull_ddna.tif")
}

fn glass() -> TestMtlMaterial {
    TestMtlMaterial::new("glass").shader("GlassPBR")
}

fn parse(bytes: &[u8]) -> Result<MaterialLibrary, MaterialError> {
    MaterialLibrary::parse("objects/test/cube", bytes)
}

#[test]
fn sub_materials_are_read_in_id_order_with_colours_opacity_and_texture_slots() -> TestResult {
    let file = TestMtl::multi(&[glass(), hull()]).with_default_palette("Libs/Palette/test");

    let library = parse(&file.cryxml())?;

    assert!(!library.single, "a list of sub-materials");
    assert_eq!(library.path, "objects/test/cube", "the path is kept");
    assert_eq!(
        library.default_palette.as_deref(),
        Some("Libs/Palette/test"),
        "the default palette"
    );
    let names: Vec<&str> = library.materials.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["glass", "hull"], "ID order");
    let hull = library.material(1).ok_or("material 1")?;
    assert_eq!(hull.shader, "HardSurface", "shader");
    assert_eq!(hull.surface_type, "metal_dense", "surface type");
    assert_eq!(
        hull.string_gen_mask, "%NORMAL_MAP%VERTCOLORS",
        "generation mask"
    );
    assert_eq!(hull.diffuse, [0.25, 0.5, 0.75], "diffuse");
    assert_eq!(hull.specular, [1.0, 0.9, 0.8], "specular");
    assert_eq!(hull.emissive, [0.1, 0.2, 0.3], "emissive");
    assert_eq!(hull.glow, 2.0, "glow");
    assert_eq!(hull.opacity, 0.5, "opacity");
    assert_eq!(hull.shininess, 204.0, "shininess");
    assert_eq!(hull.alpha_test, 0.4, "alpha test");
    assert!(hull.flags.two_sided(), "two-sided flag");
    assert!(!hull.flags.no_draw(), "drawn");
    let slots: Vec<(&str, &str)> = hull
        .textures
        .iter()
        .map(|t| (t.map.as_str(), t.file.as_str()))
        .collect();
    assert_eq!(
        slots,
        [
            ("TexSlot1", "Objects/Test/hull_diff.tif"),
            ("TexSlot2", "Objects/Test/hull_ddna.tif")
        ],
        "texture slots in file order"
    );
    assert_eq!(library.material(2), None, "an ID past the list has none");
    Ok(())
}

#[test]
fn text_xml_and_every_cryxml_form_read_alike() -> TestResult {
    let file = TestMtl::multi(&[glass(), hull()]);
    let expected = parse(&file.text_xml())?;

    for signature in [
        CryXmlSignature::CryXmlB,
        CryXmlSignature::CryXml,
        CryXmlSignature::Cry3Sdk,
    ] {
        for byte_order in [CryXmlByteOrder::Little, CryXmlByteOrder::Big] {
            let library = parse(&file.cryxml_as(signature, byte_order))?;
            assert_eq!(library, expected, "{signature:?} {byte_order:?}");
        }
    }
    assert_eq!(expected.materials.len(), 2, "both sub-materials");
    Ok(())
}

#[test]
fn a_file_without_sub_materials_is_one_material_for_every_id() -> TestResult {
    let library = parse(&TestMtl::single(&hull()).cryxml())?;

    assert!(library.single, "one material");
    for id in [0, 1, 7, u16::MAX] {
        assert_eq!(
            library.material(id).map(|m| m.name.as_str()),
            Some("hull"),
            "ID {id}"
        );
    }
    Ok(())
}

#[test]
fn mtl_flags_written_as_negative_numbers_keep_their_bits() -> TestResult {
    let flags = 0x8000_0402;

    let library = parse(&TestMtl::single(&hull().flags(flags)).text_xml())?;

    let material = library.material(0).ok_or("material")?;
    assert_eq!(material.flags.0, flags, "the 32 bits");
    assert!(material.flags.no_draw() && material.flags.two_sided());
    assert!(material.hidden(), "a no-draw material is hidden");
    Ok(())
}

#[test]
fn missing_malformed_and_non_finite_values_read_as_the_defaults() -> TestResult {
    let material = TestMtlMaterial::new("odd")
        .attribute("Diffuse", "1,2")
        .attribute("Specular", "a,b,c")
        .attribute("Emissive", "NaN,0,0")
        .attribute("Opacity", "inf")
        .attribute("Shininess", "")
        .attribute("MtlFlags", "not a number");

    let library = parse(&TestMtl::single(&material).cryxml())?;

    let odd = library.material(0).ok_or("material")?;
    assert_eq!(odd.diffuse, [1.0; 3], "two components are no colour");
    assert_eq!(odd.specular, [0.04; 3], "letters are no colour");
    assert_eq!(odd.emissive, [0.0; 3], "NaN is no number");
    assert_eq!(odd.opacity, 1.0, "infinity is no number");
    assert_eq!(odd.shininess, 10.0, "an empty value is none");
    assert_eq!(odd.flags.0, 0, "no flags");
    assert_eq!(odd.glow, 0.0, "no glow");
    assert!(odd.textures.is_empty(), "no textures");
    Ok(())
}

#[test]
fn textures_without_a_file_are_left_out() -> TestResult {
    let material = TestMtlMaterial::new("m")
        .texture("TexSlot1", "")
        .texture("TexSlot2", "$TintPaletteDecal");

    let library = parse(&TestMtl::single(&material).text_xml())?;

    let slots: Vec<&str> = library
        .materials
        .iter()
        .flat_map(|m| &m.textures)
        .map(|t| t.file.as_str())
        .collect();
    assert_eq!(
        slots,
        ["$TintPaletteDecal"],
        "placeholders stay, empty ones go"
    );
    Ok(())
}

#[test]
fn a_file_that_is_no_material_or_no_xml_is_a_typed_error() {
    let not_material = TestElement::new("Library").to_xml().into_bytes();
    assert!(matches!(
        parse(&not_material),
        Err(MaterialError::NotMaterial { found }) if found == "Library"
    ));
    assert!(matches!(parse(b"<Material"), Err(MaterialError::Xml(_))));
    assert!(matches!(parse(b""), Err(MaterialError::CryXml(_))));
    assert!(matches!(
        parse(b"CryXmlB\0\x01\x02"),
        Err(MaterialError::CryXml(_))
    ));
}

#[test]
fn more_sub_materials_or_texture_slots_than_the_limits_are_refused() {
    let many = vec![glass(); MAX_SUB_MATERIALS + 1];
    assert!(matches!(
        parse(&TestMtl::multi(&many).cryxml()),
        Err(MaterialError::TooMany {
            what: "sub-materials",
            ..
        })
    ));
    let exactly = vec![glass(); MAX_SUB_MATERIALS];
    assert!(
        parse(&TestMtl::multi(&exactly).cryxml()).is_ok(),
        "the limit itself"
    );

    let mut crowded = TestMtlMaterial::new("crowded");
    for slot in 0..=MAX_TEXTURE_SLOTS {
        crowded = crowded.texture(&format!("TexSlot{slot}"), "a.tif");
    }
    assert!(matches!(
        parse(&TestMtl::single(&crowded).cryxml()),
        Err(MaterialError::TooMany {
            what: "texture slots in one material",
            ..
        })
    ));
}

#[test]
fn nested_sub_materials_are_not_followed() -> TestResult {
    let nested = TestElement::new("Material")
        .attribute("Name", "outer")
        .child(
            TestElement::new("SubMaterials").child(
                TestElement::new("Material")
                    .attribute("Name", "inner")
                    .child(
                        TestElement::new("SubMaterials")
                            .child(TestElement::new("Material").attribute("Name", "innermost")),
                    ),
            ),
        )
        .to_xml();

    let library = parse(nested.as_bytes())?;

    let names: Vec<&str> = library.materials.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["inner"], "only the first level counts");
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..600)) {
        let _ignored = parse(&bytes);
    }

    #[test]
    fn damaged_cryxml_never_panics(cut in 0_usize..800, at in 0_usize..800, value in any::<u8>()) {
        let mut bytes = TestMtl::multi(&[glass(), hull()]).cryxml();
        if let Some(byte) = bytes.get_mut(at) {
            *byte = value;
        }
        bytes.truncate(cut.max(1));
        let _ignored = parse(&bytes);
    }
}
