// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The head of a CHF character with what it wears: eyes, teeth, hair, and the materials the
//! character chose, exported as one GLB with textures.

use std::error::Error;
use std::str::FromStr;

use p4kx_assembly::character::{
    BODY_COLOR, Chf, EYE_COLOR, Guid, HAIR_DYE_COLOR, lowered_hash, name_hash, skin_attachment,
};
use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, ForgeItems, Head, LoadoutResolver,
    ModelProvider, PartOrigin,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, MaterialMode};
use p4kx_testkit::gltf::material::AlphaMode;
use p4kx_testkit::{
    DRESSED_BROWS, DRESSED_EYES, DRESSED_GONE_HAIR, DRESSED_HAIR, DRESSED_HAIR_OPACITY,
    DRESSED_HEAD_DIFFUSE, DRESSED_HEAD_MATERIAL, DRESSED_UNKNOWN_MATERIAL, HEAD_REGIONS,
    HEAD_VERTICES, MALE_BODY_TYPE, MALE_RIG_HASH, SplitDds, TestChf, TestEntry, TestItemPort,
    TestMaterial, TestSubMaterial, TestTextureFormat, check_glb, dressed_entries, head_rig,
};

use crate::assembly_support::{TestResult, fixture};

fn stored(guid: u128) -> Result<[u8; 16], Box<dyn Error>> {
    let text = format!("{guid:032x}");
    let dashed = format!(
        "{}-{}-{}-{}-{}",
        &text[..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..]
    );
    Ok(Guid::from_str(&dashed)?.stored())
}

fn port(
    name: &str,
    item: u128,
    children: Vec<TestItemPort>,
) -> Result<TestItemPort, Box<dyn Error>> {
    Ok(TestItemPort {
        port: lowered_hash(name),
        item: stored(item)?,
        tail_count: 0,
        children,
    })
}

const fn colour(name: u32, rgba: [u8; 4]) -> (u32, [u8; 4]) {
    (name, rgba)
}

fn material(
    port: &str,
    base: u128,
    sub: &str,
    floats: Vec<(u32, u32)>,
    colors: Vec<(u32, [u8; 4])>,
    textures: Vec<(u8, [u8; 16])>,
) -> Result<TestMaterial, Box<dyn Error>> {
    Ok(TestMaterial {
        attachment: skin_attachment(port),
        base_material: stored(base)?,
        flags: 0,
        sub_materials: vec![TestSubMaterial {
            name: lowered_hash(sub),
            textures,
            floats,
            colors,
        }],
    })
}

fn character() -> Result<Chf, Box<dyn Error>> {
    character_with(Vec::new())
}

fn character_with(more: Vec<TestMaterial>) -> Result<Chf, Box<dyn Error>> {
    let items = vec![
        port("hair_itemport", DRESSED_HAIR, Vec::new())?,
        port("eyes_itemport", DRESSED_EYES, Vec::new())?,
        port("eyelashes_itemport", DRESSED_BROWS, Vec::new())?,
        port("beard_itemport", DRESSED_GONE_HAIR, Vec::new())?,
        port("eyebrow_itemport", DRESSED_HAIR, Vec::new())?,
        port("hat_itemport", 0x999, Vec::new())?,
    ];
    let head = port("head_itemport", 0x900, items)?;
    let test = TestChf {
        body_type: MALE_BODY_TYPE,
        dna_hashes: Some((MALE_RIG_HASH, 0)),
        face_parts: (0..HEAD_REGIONS)
            .map(|part| {
                if part == 3 {
                    vec![(65535, 2), (0, 0)]
                } else {
                    vec![(0, 0), (0, 0)]
                }
            })
            .collect(),
        ports: Some(port("body_itemport", 0x901, vec![head])?),
        materials: head_materials()?.into_iter().chain(more).collect(),
        ..TestChf::v8()
    };
    Ok(Chf::read(&test.to_bytes())?)
}

fn head_materials() -> Result<Vec<TestMaterial>, Box<dyn Error>> {
    let float = |name: &str, value: f32| (name_hash(name), value.to_bits());
    Ok(vec![
        material(
            "head_itemport",
            DRESSED_HEAD_MATERIAL,
            "shader_head",
            Vec::new(),
            vec![colour(BODY_COLOR, [255, 0, 0, 255])],
            Vec::new(),
        )?,
        material(
            "eyes_itemport",
            0,
            "shader_eyeInner",
            Vec::new(),
            vec![colour(EYE_COLOR, [0, 0, 255, 255])],
            Vec::new(),
        )?,
        material(
            "hair_itemport",
            0,
            "hair_test_m",
            vec![float("BaseMelanin", 1.0), float("DyeAmount", 1.0)],
            vec![colour(HAIR_DYE_COLOR, [0, 255, 0, 255])],
            vec![(4, stored(0x31)?)],
        )?,
        material(
            "eyebrow_itemport",
            DRESSED_UNKNOWN_MATERIAL,
            "hair_test_m",
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?,
    ])
}

fn archive_entries() -> Vec<TestEntry> {
    let rig = head_rig(HEAD_VERTICES, MALE_RIG_HASH).build();
    let mut entries = dressed_entries(Some(rig));
    let texture = SplitDds::new(TestTextureFormat::LegacyBgra8, 4, 4).mips(1);
    for file in [DRESSED_HEAD_DIFFUSE, DRESSED_HAIR_OPACITY] {
        let dds = file.replace('/', "\\").replace(".tif", ".dds");
        entries.extend(texture.entries(&format!("Data\\{dds}")));
    }
    entries
}

struct Dressed {
    armour: Armour,
    glb: p4kx_testkit::CheckedGlb,
    summary: p4kx_gltf::GlbSummary,
}

fn dress(entries: Vec<TestEntry>) -> Result<Dressed, Box<dyn Error>> {
    dress_character(entries, &character()?)
}

fn dress_character(entries: Vec<TestEntry>, chf: &Chf) -> Result<Dressed, Box<dyn Error>> {
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    assert_eq!(
        items.material_count(),
        1,
        "the lookup table lists one material"
    );
    let head = Head::dressed(&chf.character, &mut items)?;
    let models = ArchiveModels::new(&archive);
    let armour = Armour::build(
        head.variant(),
        head.pieces(),
        &models,
        &AssemblyOptions::default(),
    )?;
    let options = GlbOptions::default().with_materials(MaterialMode::Textures);
    let mut bytes = Vec::new();
    let summary = armour.write_glb_with(&options, &models.textures(), &mut bytes)?;
    Ok(Dressed {
        armour,
        glb: check_glb(&bytes)?,
        summary,
    })
}

fn named<'a>(dressed: &'a Dressed, name: &str) -> Result<&'a ArmourPart, Box<dyn Error>> {
    Ok(dressed
        .armour
        .parts()
        .iter()
        .find(|part| part.name() == name)
        .ok_or_else(|| format!("no part {name}"))?)
}

fn factor(dressed: &Dressed, name: &str) -> Result<([f32; 4], bool, AlphaMode), Box<dyn Error>> {
    let found = dressed
        .glb
        .document
        .materials()
        .find(|candidate| candidate.name() == Some(name))
        .ok_or_else(|| format!("no material {name}"))?;
    let pbr = found.pbr_metallic_roughness();
    Ok((
        pbr.base_color_factor(),
        pbr.base_color_texture().is_some(),
        found.alpha_mode(),
    ))
}

fn near(found: [f32; 4], wanted: [f32; 3]) -> bool {
    found
        .iter()
        .zip(wanted)
        .all(|(found, wanted)| (found - wanted).abs() < 1e-3)
}

#[test]
fn the_head_wears_its_teeth_eyes_and_hair_and_what_it_lacks_is_listed() -> TestResult {
    let dressed = dress(archive_entries())?;

    let names: Vec<&str> = dressed
        .armour
        .parts()
        .iter()
        .map(ArmourPart::name)
        .collect();
    assert_eq!(
        names,
        [
            "body",
            "head",
            "teeth",
            "hair_itemport",
            "eyes_itemport",
            "eyebrow_itemport"
        ],
        "the items come in the order of the character's port tree"
    );
    assert_eq!(dressed.armour.count(PartOrigin::Piece), 5);
    let issues: Vec<(&str, &str)> = dressed
        .armour
        .issues()
        .iter()
        .filter(|issue| !matches!(issue.kind().code(), "bones_not_in_skeleton" | "not_skinned"))
        .map(|issue| (issue.name(), issue.kind().code()))
        .collect();
    assert!(
        issues.contains(&("beard_itemport", "geometry_missing")),
        "{issues:?}"
    );
    assert!(
        issues.contains(&("hat_itemport", "piece_not_found")),
        "{issues:?}"
    );
    assert!(
        issues.contains(&("eyebrow_itemport", "material_choice_unknown")),
        "{issues:?}"
    );
    assert!(
        issues.contains(&("hair_itemport", "look_ignored")),
        "{issues:?}"
    );
    assert!(
        !issues.iter().any(|(name, _)| *name == "eyelashes_itemport"),
        "an Entity without geometry is no piece: {issues:?}"
    );
    Ok(())
}

#[test]
fn the_materials_take_the_skin_tone_the_iris_and_the_dye_of_the_character() -> TestResult {
    let dressed = dress(archive_entries())?;

    let (head, head_textured, _) = factor(&dressed, "shader_head")?;
    let (eyes, _, _) = factor(&dressed, "shader_eyeInner")?;
    let (hair, hair_textured, hair_alpha) = factor(&dressed, "hair_test_m")?;

    assert!(
        near(head, [1.0, 0.0, 0.0]),
        "{head:?}: the skin tone rebases the head map"
    );
    assert!(head_textured, "the head keeps its diffuse map");
    assert!(near(eyes, [0.0, 0.0, 1.0]), "{eyes:?}: the iris colour");
    assert!(near(hair, [0.0, 1.0, 0.0]), "{hair:?}: a full dye");
    assert!(hair_textured, "the opacity map");
    assert_eq!(hair_alpha, AlphaMode::Mask, "hair cards are cut out");
    assert!(dressed.summary.textures >= 2, "{:?}", dressed.summary);
    assert_eq!(
        dressed.glb.document.skins().len(),
        1,
        "one skin for all pieces"
    );
    Ok(())
}

#[test]
fn the_body_takes_the_base_material_and_skin_tone_the_character_chose_for_it() -> TestResult {
    let body_choice = material(
        "body_itemport",
        DRESSED_HEAD_MATERIAL,
        "shader_head",
        Vec::new(),
        vec![colour(BODY_COLOR, [0, 0, 255, 255])],
        Vec::new(),
    )?;
    let dressed = dress_character(archive_entries(), &character_with(vec![body_choice])?)?;

    let body = named(&dressed, "body")?;
    let library = body
        .materials()
        .library()
        .ok_or("the body has no library")?;
    assert!(
        library.materials.iter().all(|skin| near(
            [skin.diffuse[0], skin.diffuse[1], skin.diffuse[2], 1.0],
            [0.0, 0.0, 1.0]
        )),
        "every body skin takes the skin tone: {:?}",
        library.materials
    );
    assert_eq!(dressed.armour.count(PartOrigin::Body), 1, "no second body");
    assert_eq!(
        dressed.armour.count(PartOrigin::Piece),
        5,
        "no part of its own"
    );
    Ok(())
}

#[test]
fn a_character_that_chose_nothing_for_its_body_keeps_the_default_body() -> TestResult {
    let dressed = dress(archive_entries())?;

    let body = named(&dressed, "body")?;
    let library = body
        .materials()
        .library()
        .ok_or("the body has no library")?;
    assert!(
        library.materials.iter().all(|skin| near(
            [skin.diffuse[0], skin.diffuse[1], skin.diffuse[2], 1.0],
            [0.5; 3]
        )),
        "{:?}",
        library.materials
    );
    Ok(())
}

#[test]
fn what_grows_on_the_head_follows_the_shaped_head() -> TestResult {
    let dressed = dress(archive_entries())?;
    let fixture = fixture(archive_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let plain = ArchiveModels::new(&archive).model(
        "Objects/Characters/Human/heads/shared/hair_v2/sc/male/m_hair_test.skin",
        0,
    )?;

    let before = &plain.model.lods[0].meshes[0].positions;
    let after = &named(&dressed, "hair_itemport")?.model().lods[0].meshes[0].positions;

    let moved: Vec<f32> = before
        .iter()
        .zip(after)
        .map(|(was, now)| {
            was.iter()
                .zip(now)
                .map(|(was, now)| (was - now).powi(2))
                .sum::<f32>()
                .sqrt()
        })
        .collect();
    let moved_count = moved.iter().filter(|distance| **distance > 0.005).count();
    assert!(moved_count > 0, "the hair at the moved corner follows");
    assert!(
        moved_count < moved.len() / 2,
        "the rest of the hair stays: {moved:?}"
    );
    Ok(())
}
