// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The colours of the layered materials of real ships: how many materials are drawn dark.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use p4kx_assembly::{ArchiveModels, Assembly, AssemblyOptions, ForgePalettes, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::base_colour_factor;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 3] = ["RSI_Polaris", "AEGS_Gladius", "RSI_Constellation_Andromeda"];
const DARK: f32 = 0.05;

fn luminance([red, green, blue]: [f32; 3]) -> f32 {
    0.0722_f32.mul_add(blue, 0.2126_f32.mul_add(red, 0.7152 * green))
}

#[derive(Default)]
struct Census {
    materials: usize,
    dark: usize,
    layered: usize,
    layered_dark: usize,
    names: Vec<String>,
}

fn census(assembly: &Assembly) -> Census {
    let mut seen: BTreeSet<(usize, usize, usize)> = BTreeSet::new();
    let mut found = Census::default();
    let mut by_name: BTreeMap<String, f32> = BTreeMap::new();
    for part in assembly.parts() {
        let (Some(model), Some(library)) = (part.model(), part.materials().library()) else {
            continue;
        };
        let Some(lod) = model.lods.first() else {
            continue;
        };
        let ids: BTreeSet<u16> = lod
            .meshes
            .iter()
            .flat_map(|mesh| &mesh.submeshes)
            .map(|submesh| submesh.material_id)
            .collect();
        for id in ids {
            let position = if library.single { 0 } else { usize::from(id) };
            let Some(material) = library.materials.get(position) else {
                continue;
            };
            if material.hidden() || material.shader.eq_ignore_ascii_case("MeshDecal") {
                continue;
            }
            let key = (
                std::ptr::from_ref(library) as usize,
                part.palette().map_or(0, |palette| std::ptr::from_ref(palette) as usize),
                position,
            );
            if !seen.insert(key) {
                continue;
            }
            let tinted = part.palette().and_then(|palette| palette.apply(material));
            let material = tinted.as_ref().unwrap_or(material);
            let shade = luminance(base_colour_factor(material));
            found.materials += 1;
            let layered = material.shader.eq_ignore_ascii_case("HardSurface");
            if layered {
                found.layered += 1;
            }
            if shade < DARK {
                found.dark += 1;
                if layered {
                    found.layered_dark += 1;
                }
                by_name.insert(material.name.clone(), shade);
            }
        }
    }
    found.names = by_name
        .into_iter()
        .map(|(name, shade)| format!("{name} {shade:.3}"))
        .collect();
    found
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the share of dark materials is the measurement, printed in one run"
)]
fn the_share_of_dark_materials_of_the_ships_is_measured() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let palettes = ForgePalettes::new(&forge);
    for name in SHIPS {
        let mut resolver = LoadoutResolver::new(&forge);
        let Ok(record) = resolver.entities().find(name) else {
            println!("{name}: not in this Archive");
            continue;
        };
        let tree = resolver.resolve(&record)?;
        let assembly =
            Assembly::build_with_palettes(&tree, &models, &palettes, &AssemblyOptions::default())?;
        let found = census(&assembly);
        println!(
            "{name}: {} materials, {} dark (under {DARK}), {} layered of which {} dark",
            found.materials, found.dark, found.layered, found.layered_dark
        );
        for line in found.names.iter().take(60) {
            println!("  dark: {line}");
        }
    }
    Ok(())
}
