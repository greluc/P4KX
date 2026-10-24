// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The colours of the layered materials of real ships: how many materials are drawn dark with
//! and without the colours of their layers.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use p4kx_assembly::{ArchiveModels, Assembly, AssemblyOptions, ForgePalettes, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::base_colour_factor;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 3] = ["RSI_Polaris", "AEGS_Gladius", "RSI_Constellation_Andromeda"];
const DARK: f32 = 0.05;
const BLACK: f32 = 0.005;

fn luminance([red, green, blue]: [f32; 3]) -> f32 {
    0.0722_f32.mul_add(blue, 0.2126_f32.mul_add(red, 0.7152 * green))
}

#[derive(Default)]
struct Census {
    materials: usize,
    layered: usize,
    layers_resolved: usize,
    black_before: usize,
    black_after: usize,
    lightened: usize,
    darkened: usize,
    dark_before: usize,
    dark_after: usize,
    layered_dark_before: usize,
    layered_dark_after: usize,
    unresolved: BTreeSet<String>,
    still_dark: BTreeMap<String, f32>,
}

#[expect(
    clippy::too_many_lines,
    reason = "one pass counts every column of the report"
)]
fn census(assembly: &Assembly) -> Census {
    let mut seen: BTreeSet<(usize, usize, usize)> = BTreeSet::new();
    let mut found = Census::default();
    for part in assembly.parts() {
        let (Some(model), Some(library)) = (part.model(), part.materials().library()) else {
            continue;
        };
        found
            .unresolved
            .extend(library.unresolved_layers.iter().cloned());
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
                part.palette()
                    .map_or(0, |palette| std::ptr::from_ref(palette) as usize),
                position,
            );
            if !seen.insert(key) {
                continue;
            }
            let tinted = part.palette().and_then(|palette| palette.apply(material));
            let material = tinted.as_ref().unwrap_or(material);
            let mut plain = material.clone();
            plain.layers.clear();
            let shade_before = luminance(base_colour_factor(&plain));
            let shade_after = luminance(base_colour_factor(material));
            let before = shade_before < DARK;
            let after = shade_after < DARK;
            found.black_before += usize::from(shade_before < BLACK);
            found.black_after += usize::from(shade_after < BLACK);
            let layered = !material.layers.is_empty();
            found.materials += 1;
            found.dark_before += usize::from(before);
            found.dark_after += usize::from(after);
            found.lightened += usize::from(before && !after);
            found.darkened += usize::from(after && !before);
            if layered {
                found.layered += 1;
                found.layered_dark_before += usize::from(before);
                found.layered_dark_after += usize::from(after);
                found.layers_resolved +=
                    usize::from(material.layers.iter().all(|layer| layer.surface.is_some()));
            }
            if after {
                found.still_dark.insert(
                    material.name.clone(),
                    luminance(base_colour_factor(material)),
                );
            }
        }
    }
    found
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the share of dark materials is the measurement, printed in one run"
)]
fn the_layers_lighten_the_dark_materials_of_the_ships() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let palettes = ForgePalettes::new(&forge);
    let mut measured = 0;
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
            "{name}: {} materials; dark (luminance under {DARK}) {} before, {} after; \
             {} layered, of which dark {} before, {} after; {} with all layers read; \
             {} layer files missing; near black (under {BLACK}) {} before, {} after; {} dark materials lightened, {} others darkened",
            found.materials,
            found.dark_before,
            found.dark_after,
            found.layered,
            found.layered_dark_before,
            found.layered_dark_after,
            found.layers_resolved,
            found.unresolved.len(),
            found.black_before,
            found.black_after,
            found.lightened,
            found.darkened
        );
        for path in found.unresolved.iter().take(10) {
            println!("  missing layer file: {path}");
        }
        for (material, shade) in found.still_dark.iter().take(60) {
            println!("  still dark: {material} {shade:.3}");
        }
        assert!(
            found.lightened > 0,
            "{name}: the layers lighten some materials"
        );
        measured += 1;
    }
    assert!(measured > 0, "a ship is in the Archive");
    Ok(())
}
