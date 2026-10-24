// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! M9 verification: a documented small ship, medium ship, capital ship, ground vehicle and weapon
//! of a real Archive are assembled, written as GLB with textures and palettes, and checked for
//! plausibility by ranges and properties, never by exact counts.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::time::{Duration, Instant};

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, ArmourPart, Assembly, AssemblyOptions, BodyVariant,
    EntityCatalogue, EntityKind, EntitySummary, ForgePalettes, LoadoutResolver, Placement,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, GlbSummary, MaterialMode};
use p4kx_model::Model;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

use super::real_entity_interior::{Bounds, hull_bounds};

type TestResult = Result<(), Box<dyn Error>>;

struct Case {
    class_name: &'static str,
    size: &'static str,
    lod: u32,
    least_parts: usize,
    shares_meshes: bool,
}

const CASES: [Case; 5] = [
    Case {
        class_name: "AEGS_Gladius",
        size: "small ship",
        lod: 0,
        least_parts: 20,
        shares_meshes: true,
    },
    Case {
        class_name: "RSI_Constellation_Andromeda",
        size: "medium ship",
        lod: 0,
        least_parts: 40,
        shares_meshes: true,
    },
    Case {
        class_name: "AEGS_Idris_P",
        size: "capital ship",
        lod: 2,
        least_parts: 100,
        shares_meshes: true,
    },
    Case {
        class_name: "TMBL_Cyclone",
        size: "ground vehicle",
        lod: 0,
        least_parts: 10,
        shares_meshes: true,
    },
    Case {
        class_name: "",
        size: "weapon",
        lod: 0,
        least_parts: 1,
        shares_meshes: false,
    },
];

const WEAPONS: [&str; 3] = [
    "BEHR_LaserCannon_S3",
    "KLWE_LaserRepeater_S3",
    "BEHR_BallisticCannon_S3",
];

const TEXTURE_MIP: u32 = 3;
const MAX_SKIPPED_SHARE_PERCENT: usize = 10;
const MAX_BOX_SHARE_PERCENT: usize = 15;
const MAX_ORIGIN_SHARE_PERCENT: usize = 60;
const MIN_NEAR_HULL_SHARE_PERCENT: usize = 85;
const HULL_MARGIN_RATIO: f32 = 0.5;
const MAX_TEXTURE_PIXELS: u64 = 400 * 1024 * 1024;
const MAX_GLB_BYTES: u64 = 1 << 30;
const MAX_BUILD: Duration = Duration::from_secs(600);
const BOX_VERTICES: usize = 24;

fn is_box(model: &Model, lod: u32) -> bool {
    let Some(level) = model
        .lods
        .iter()
        .find(|candidate| candidate.level == lod)
        .or_else(|| model.lods.first())
    else {
        return false;
    };
    level.vertex_count() > 0 && level.vertex_count() <= BOX_VERTICES && level.triangle_count() <= 12
}

fn near_hull(point: [f32; 3], (min, max): &Bounds) -> bool {
    let extent = min
        .iter()
        .zip(max)
        .map(|(low, high)| high - low)
        .fold(0.0_f32, f32::max);
    let margin = extent * HULL_MARGIN_RATIO;
    point
        .iter()
        .zip(min.iter().zip(max))
        .all(|(value, (low, high))| *value >= low - margin && *value <= high + margin)
}

struct Measured {
    parts: usize,
    modelled: usize,
    skipped: usize,
    boxes: usize,
    origin: usize,
    placed_children: usize,
    near_hull: usize,
    misparented: usize,
    summary: GlbSummary,
    built: Duration,
    written: Duration,
}

#[expect(
    clippy::too_many_lines,
    reason = "one scenario from building to measuring"
)]
fn measure(
    archive: &Archive,
    class_name: &str,
    lod: u32,
    mode: MaterialMode,
    dir: &std::path::Path,
) -> Result<Measured, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(archive);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let record = resolver.entities().find(class_name)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(archive);
    let palettes = ForgePalettes::new(&forge);
    let started = Instant::now();
    let options = AssemblyOptions::default().with_lod(lod);
    let assembly = Assembly::build_with_palettes(&tree, &models, &palettes, &options)?;
    let built = started.elapsed();

    let path = dir.join(format!("{class_name}-{mode:?}.glb"));
    let started = Instant::now();
    let glb_options = GlbOptions::default()
        .with_lod(lod)
        .with_materials(mode)
        .with_texture_mip(TEXTURE_MIP);
    let mut writer = BufWriter::new(File::create(&path)?);
    let summary = assembly.write_glb_with(&glb_options, &models.textures(), &mut writer)?;
    drop(writer);
    let written = started.elapsed();
    let bytes = std::fs::read(&path)?;
    std::fs::remove_file(&path)?;
    check_glb(&bytes)?;

    let hull = assembly
        .root()
        .and_then(|part| part.model())
        .and_then(hull_bounds);
    let mut measured = Measured {
        parts: assembly.parts().len(),
        modelled: assembly.modelled_parts(),
        skipped: assembly
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .count(),
        boxes: 0,
        origin: 0,
        placed_children: 0,
        near_hull: 0,
        misparented: 0,
        summary,
        built,
        written,
    };
    for part in assembly.parts().iter().skip(1) {
        let parent = part.parent().and_then(|index| assembly.parts().get(index));
        measured.misparented += usize::from(parent.is_none_or(|parent| {
            let expected = parent.world().then(part.local());
            !expected
                .rows
                .iter()
                .flatten()
                .zip(part.world().rows.iter().flatten())
                .all(|(a, b)| (a - b).abs() < 1e-3)
        }));
        measured.boxes += usize::from(part.model().is_some_and(|model| is_box(model, lod)));
        if part.model().is_none() {
            continue;
        }
        measured.placed_children += 1;
        measured.origin += usize::from(matches!(part.placement(), Placement::Origin));
        if let Some(bounds) = &hull {
            measured.near_hull += usize::from(near_hull(part.world().translation(), bounds));
        }
    }
    Ok(measured)
}

fn weapon_class(archive: &Archive) -> Result<String, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let catalogue = EntityCatalogue::build(&forge, &|| false).ok_or("stopped")?;
    let weapons: Vec<&str> = catalogue
        .entries()
        .iter()
        .filter(|summary| summary.kind() == EntityKind::Weapon)
        .map(EntitySummary::class_name)
        .collect();
    let chosen = WEAPONS
        .iter()
        .find(|name| weapons.iter().any(|class| class.eq_ignore_ascii_case(name)))
        .copied()
        .or_else(|| weapons.first().copied())
        .ok_or("no weapon in the Archive")?;
    Ok(chosen.to_owned())
}

#[expect(
    clippy::print_stdout,
    reason = "the counts and times are what the pull request reports"
)]
fn report(class_name: &str, size: &str, mode: MaterialMode, measured: &Measured) {
    let summary = &measured.summary;
    println!(
        "{class_name} ({size}) {mode:?} mip {TEXTURE_MIP}: {} parts, {} modelled, {} skipped, {} box-like, \
         {} at the origin of {} placed, {} near the hull; {} nodes, {} meshes ({} shared), \
         {} materials ({} shared), {} images ({} px, {} shared references, {} fallbacks), \
         {} triangles, {} bytes; built in {:?}, written in {:?}",
        measured.parts,
        measured.modelled,
        measured.skipped,
        measured.boxes,
        measured.origin,
        measured.placed_children,
        measured.near_hull,
        summary.nodes,
        summary.meshes,
        summary.shared_meshes,
        summary.materials,
        summary.shared_materials,
        summary.textures,
        summary.texture_pixels,
        summary.shared_textures,
        summary.texture_fallbacks.len(),
        summary.triangles,
        summary.bytes,
        measured.built,
        measured.written,
    );
}

#[expect(clippy::too_many_lines, reason = "one assertion per property")]
fn verify(case: &Case, class_name: &str, mode: MaterialMode, measured: &Measured) {
    let summary = &measured.summary;
    let name = format!("{class_name} ({})", case.size);
    assert!(
        measured.parts >= case.least_parts,
        "{name}: {} parts",
        measured.parts
    );
    assert!(measured.modelled > 0, "{name}: no part has a model");
    assert_eq!(
        measured.misparented, 0,
        "{name}: parts whose world transform is not their local one in their parent's"
    );
    assert!(
        measured.skipped * 100 <= (measured.parts + measured.skipped) * MAX_SKIPPED_SHARE_PERCENT,
        "{name}: {} of {} parts skipped",
        measured.skipped,
        measured.parts
    );
    assert!(
        measured.boxes * 100 <= measured.placed_children.max(1) * MAX_BOX_SHARE_PERCENT,
        "{name}: {} of {} parts are boxes",
        measured.boxes,
        measured.placed_children
    );
    if measured.placed_children >= 10 {
        assert!(
            measured.origin * 100 <= measured.placed_children * MAX_ORIGIN_SHARE_PERCENT,
            "{name}: {} of {} parts sit at the origin of their parent",
            measured.origin,
            measured.placed_children
        );
        assert!(
            measured.near_hull * 100 >= measured.placed_children * MIN_NEAR_HULL_SHARE_PERCENT,
            "{name}: {} of {} parts are near the hull",
            measured.near_hull,
            measured.placed_children
        );
    }
    if case.shares_meshes {
        assert!(summary.shared_meshes > 0, "{name}: no mesh is shared");
    }
    assert!(
        summary.bytes <= MAX_GLB_BYTES,
        "{name}: {} bytes",
        summary.bytes
    );
    assert!(
        summary.texture_pixels <= MAX_TEXTURE_PIXELS,
        "{name}: {} texture pixels",
        summary.texture_pixels
    );
    assert!(
        measured.built + measured.written <= MAX_BUILD,
        "{name}: too slow"
    );
    if mode == MaterialMode::Textures {
        assert!(summary.textures > 0, "{name}: no image embedded");
        assert!(
            summary.texture_fallbacks.len() * 2
                <= summary.textures + summary.texture_fallbacks.len(),
            "{name}: {} textures fell back",
            summary.texture_fallbacks.len()
        );
    }
}

const ARMOUR_LOADOUT: &str =
    "Scripts/Loadouts/Character/Human/Armor/NPC/Criminal/Pyro/Criminal_Gang_Guard_Heavy_Boss.xml";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn worn_armour_is_real_geometry_and_not_boxes() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let tree = resolver.resolve_file(ARMOUR_LOADOUT)?;
    let pieces = resolver.worn(&tree);
    for variant in BodyVariant::ALL {
        let armour = Armour::build(variant, &pieces, &models, &AssemblyOptions::default())?;
        let mut bytes = Vec::new();
        let summary = armour.write_glb(&GlbOptions::default(), &mut bytes)?;
        check_glb(&bytes)?;
        let skipped = armour
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .count();
        let boxes: Vec<&str> = armour
            .parts()
            .iter()
            .filter(|part| is_box(part.model(), 0))
            .map(ArmourPart::name)
            .collect();
        assert!(
            skipped * 5 <= armour.parts().len(),
            "{variant:?}: {skipped} skipped"
        );
        assert!(boxes.is_empty(), "{variant:?}: boxes {boxes:?}");
        assert!(
            summary.triangles > 10_000 && summary.bones > 100,
            "{variant:?}: {} triangles, {} bones",
            summary.triangles,
            summary.bones
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn documented_entities_assemble_into_valid_complete_glbs() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let dir = std::env::temp_dir().join("m9-133-real");
    std::fs::create_dir_all(&dir)?;
    let weapon = weapon_class(&archive)?;
    for case in &CASES {
        let class_name = if case.class_name.is_empty() {
            weapon.as_str()
        } else {
            case.class_name
        };
        for mode in [MaterialMode::Colours, MaterialMode::Textures] {
            let measured = measure(&archive, class_name, case.lod, mode, &dir)?;
            report(class_name, case.size, mode, &measured);
            verify(case, class_name, mode, &measured);
        }
    }
    std::fs::remove_dir(&dir)?;
    Ok(())
}
