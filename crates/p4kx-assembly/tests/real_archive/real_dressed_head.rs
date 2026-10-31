// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters and the mission givers the game ships, as heads with their eyes, hair
//! and materials, exported as one valid GLB with textures.

use std::error::Error;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

use p4kx_assembly::character::{BODY_COLOR, Chf, skin_attachment};
use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, Expressions, ForgeItems, Head,
    LoadoutResolver, ModelProvider, PartOrigin,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, MaterialMode};
use p4kx_model::srgb8_to_linear;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPPED: &str = "libs\\charactercustomizer\\";
const DEFAULTS: [&str; 2] = ["masculinedefault.chf", "femininedefault.chf"];
const ALWAYS: [&str; 3] = ["head", "eyes_itemport", "teeth"];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn every_shipped_character_gets_a_head_with_eyes_hair_and_textured_materials() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let textures = models.textures();
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    assert!(items.material_count() > 20, "the lookup table is read");
    let (mut checked, mut dressed) = (0_usize, 0_usize);
    for file in archive.entries() {
        let path = file.path().to_ascii_lowercase();
        let is_chf = Path::new(&path)
            .extension()
            .is_some_and(|extension| extension == "chf");
        if !is_chf || !path.contains(SHIPPED) {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;
        checked += 1;
        let Ok(head) = Head::dressed(&chf.character, &mut items) else {
            continue;
        };
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default().with_lod(2),
        )?;
        let skipped: Vec<_> = armour
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .map(|issue| format!("{}: {}", issue.name(), issue.kind()))
            .collect();
        assert!(skipped.is_empty(), "{path}: {skipped:?}");
        let names: Vec<&str> = armour.parts().iter().map(ArmourPart::name).collect();
        assert_eq!(armour.count(PartOrigin::Body), 1, "{path}");
        for needed in ALWAYS {
            assert!(names.contains(&needed), "{path}: no {needed} in {names:?}");
        }

        let options = GlbOptions::default()
            .with_lod(2)
            .with_materials(MaterialMode::Textures)
            .with_texture_mip(4);
        let mut glb = Vec::new();
        let summary = armour.write_glb_with(&options, &textures, &mut glb)?;
        let checked_glb = check_glb(&glb)?;
        assert_eq!(checked_glb.document.skins().len(), 1, "{path}: one skin");
        assert!(summary.textures > 4, "{path}: {} images", summary.textures);
        dressed += 1;
    }
    println!("{checked} shipped characters, {dressed} dressed");
    assert!(checked >= 2, "the Archive ships characters");
    assert!(dressed >= 2, "the default characters are dressed");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the size and time are the measurement")]
fn the_default_characters_are_dressed_at_the_most_detailed_level_with_their_textures() -> TestResult
{
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let textures = models.textures();
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    for name in DEFAULTS {
        let file = archive
            .entries()
            .iter()
            .find(|entry| entry.path().to_ascii_lowercase().ends_with(name))
            .ok_or_else(|| format!("the Archive ships no {name}"))?;
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;

        let started = Instant::now();
        let head = Head::dressed(&chf.character, &mut items)?;
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default(),
        )?;
        let options = GlbOptions::default()
            .with_materials(MaterialMode::Textures)
            .with_texture_mip(1);
        let mut glb = Vec::new();
        let summary = armour.write_glb_with(&options, &textures, &mut glb)?;
        let elapsed = started.elapsed();
        check_glb(&glb)?;

        let names: Vec<&str> = armour.parts().iter().map(ArmourPart::name).collect();
        println!(
            "{name}: {names:?}, {} bytes, {} images of {} pixels, {} materials, {:?}",
            glb.len(),
            summary.textures,
            summary.texture_pixels,
            summary.materials,
            elapsed
        );
        for fallback in &summary.texture_fallbacks {
            println!(
                "   no {} texture {} for {}: {}",
                fallback.role, fallback.file, fallback.material, fallback.reason
            );
        }
        for issue in armour.issues().iter().filter(|issue| issue.kind().skips()) {
            println!("   skipped {}: {}", issue.name(), issue.kind());
        }
        assert!(
            armour.issues().iter().all(|issue| !issue.kind().skips()),
            "{name}"
        );
        assert!(summary.textured_materials >= 6, "{name}: {summary:?}");
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the counts and time are the measurement"
)]
#[expect(clippy::too_many_lines, reason = "the measurement reads top to bottom")]
fn the_hair_of_the_default_characters_follows_their_expressions() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    for name in DEFAULTS {
        let file = archive
            .entries()
            .iter()
            .find(|entry| entry.path().to_ascii_lowercase().ends_with(name))
            .ok_or_else(|| format!("the Archive ships no {name}"))?;
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;

        let started = Instant::now();
        let head = Head::dressed(&chf.character, &mut items)?.with_expressions(&Expressions::All);
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default(),
        )?;
        let elapsed = started.elapsed();

        let counts: Vec<(&str, usize)> = armour
            .parts()
            .iter()
            .map(|part| (part.name(), part.morphs().len()))
            .collect();
        println!("{name}: {counts:?}, {elapsed:?}");
        let of = |part: &str| counts.iter().find(|(found, _)| *found == part).map(|c| c.1);
        assert!(of("head") > Some(0), "{name}: {counts:?}");
        assert!(of("hair_itemport") > Some(0), "{name}: {counts:?}");
        assert!(of("hair_itemport") <= of("head"), "{name}: {counts:?}");
        let mut glb = Vec::new();
        armour.write_glb(&GlbOptions::default(), &mut glb)?;
        check_glb(&glb)?;

        let following = armour
            .parts()
            .iter()
            .filter(|part| !matches!(part.name(), "head" | "body" | "teeth" | "eyes_itemport"));
        let (mut targets, mut vertices) = (0_usize, 0_usize);
        for part in following {
            targets += part.morphs().len();
            vertices += part
                .morphs()
                .iter()
                .map(|t| t.vertices.len())
                .sum::<usize>();
        }
        let plain = Head::dressed(&chf.character, &mut items)?;
        let bare = Armour::build(
            plain.variant(),
            plain.pieces(),
            &models,
            &AssemblyOptions::default(),
        )?;
        let mut bare_glb = Vec::new();
        bare.write_glb(&GlbOptions::default(), &mut bare_glb)?;
        println!(
            "{name}: {targets} hair targets, {} vertices on average, {} KiB of target data, \
             GLB {} KiB with all expressions against {} KiB without",
            vertices / targets.max(1),
            vertices * 16 / 1024,
            glb.len() / 1024,
            bare_glb.len() / 1024
        );
    }
    Ok(())
}

fn skin_tone(chf: &Chf, port: &str) -> Option<[u8; 4]> {
    let hash = skin_attachment(port);
    chf.character
        .materials
        .iter()
        .find(|choice| choice.attachment == hash)?
        .sub_materials
        .iter()
        .flat_map(|sub| &sub.colors)
        .find(|colour| colour.name == BODY_COLOR)
        .map(|colour| colour.rgba)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_body_of_a_default_character_has_the_skin_tone_of_its_head() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    for name in DEFAULTS {
        let file = archive
            .entries()
            .iter()
            .find(|entry| entry.path().to_ascii_lowercase().ends_with(name))
            .ok_or_else(|| format!("the Archive ships no {name}"))?;
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;
        let tone = skin_tone(&chf, "body_itemport").ok_or("no body skin tone")?;
        assert_eq!(skin_tone(&chf, "head_itemport"), Some(tone), "{name}");

        let head = Head::dressed(&chf.character, &mut items)?;
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default(),
        )?;
        let body = armour
            .parts()
            .iter()
            .find(|part| part.origin() == PartOrigin::Body)
            .ok_or("no body")?;
        let library = body.materials().library().ok_or("no body library")?;
        let wanted = [tone[0], tone[1], tone[2]].map(srgb8_to_linear);
        let mut skins = 0;
        for skin in library
            .materials
            .iter()
            .filter(|skin| skin.shader.eq_ignore_ascii_case("HumanSkin_V2"))
        {
            let average = skin
                .public_param("SourceAverageColor")
                .ok_or("no average")?;
            let scale: Vec<f32> = (0..3)
                .map(|channel| skin.diffuse[channel] * average[channel] / wanted[channel])
                .collect();
            let (low, high) = scale
                .iter()
                .fold((f32::MAX, f32::MIN), |(low, high), value| {
                    (low.min(*value), high.max(*value))
                });
            assert!(high / low < 1.01, "{name} {}: {scale:?}", skin.name);
            skins += 1;
        }
        assert!(skins >= 2, "{name}: {skins} body skins");
    }
    Ok(())
}

/// Returns the distance from `at` to the nearest of `points`.
fn nearest(points: &[[f32; 3]], at: [f32; 3]) -> f32 {
    points
        .iter()
        .map(|point| {
            point
                .iter()
                .zip(&at)
                .map(|(from, to)| (from - to).powi(2))
                .sum::<f32>()
        })
        .fold(f32::MAX, f32::min)
        .sqrt()
}

/// Returns by how much, at most, a beard vertex of `armour` is farther from the shaped head than
/// the same vertex of the unshaped skins is from the unshaped head; `None` without head or beard.
fn beard_gap_growth(
    armour: &Armour,
    models: &ArchiveModels<'_>,
) -> Result<Option<f32>, Box<dyn Error>> {
    let part = |name: &str| armour.parts().iter().find(|part| part.name() == name);
    let (Some(head), Some(beard)) = (part("head"), part("beard_itemport")) else {
        return Ok(None);
    };
    let positions = |model: &p4kx_model::Model| {
        model
            .lods
            .first()
            .and_then(|lod| lod.meshes.first())
            .map(|mesh| mesh.positions.clone())
            .ok_or("a model without a mesh")
    };
    let raw_head = positions(&models.model(head.file(), 0)?.model)?;
    let raw_beard = positions(&models.model(beard.file(), 0)?.model)?;
    let (head, beard) = (positions(head.model())?, positions(beard.model())?);
    Ok(Some(
        raw_beard
            .iter()
            .zip(&beard)
            .map(|(raw, shaped)| nearest(&head, *shaped) - nearest(&raw_head, *raw))
            .fold(0.0_f32, f32::max),
    ))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the distances are the measurement")]
fn the_beards_of_the_shipped_characters_stay_within_a_few_millimetres_of_the_head_they_follow()
-> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    let (mut beards, mut worst) = (0_usize, 0.0_f32);
    for file in archive.entries() {
        let path = file.path().to_ascii_lowercase();
        let is_chf = Path::new(&path)
            .extension()
            .is_some_and(|extension| extension == "chf");
        if !is_chf || !path.contains(SHIPPED) {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let Ok(head) = Chf::read(&bytes)
            .map_err(Box::<dyn Error>::from)
            .and_then(|chf| Ok(Head::dressed(&chf.character, &mut items)?))
        else {
            continue;
        };
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default(),
        )?;
        let Some(growth) = beard_gap_growth(&armour, &models)? else {
            continue;
        };
        println!("{path}: the beard's gap to the head grows by {growth:.4} m at most");
        worst = worst.max(growth);
        beards += 1;
    }
    assert!(beards >= 10, "{beards} shipped beards");
    assert!(worst < 0.008, "the beards drift {worst} m from the head");
    Ok(())
}
